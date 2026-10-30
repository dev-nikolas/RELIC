//! Pruebas de Integración de Telemetría Fuera de Banda (Seis Ojos) en relic_blackwall.
//!
//! Valida:
//! 1. `PassiveMonitor` no contamina el WAL `engram.jsonl` (aislamiento estricto).
//! 2. Rollback atómico: si el oráculo o la regresión rechazan una transacción, la telemetría NO se compromete.
//! 3. Contención intra-lote: múltiples puntos para el mismo canal en la misma transacción son rechazados.
//! 4. Capacidad de la arena: saturación perezosa retorna `ArenaExhausted` sin entrar en pánico.
//! 5. Modo `Enforcing`: rechazo inmediato de anomalías sin envenenar el historial del detector.
//! 6. Transacción atómica combinada: mutación de grafo + telemetría se comprometen juntas.

use relic_blackwall::{
    AnomalyGateMode, BindingTarget, BlackwallError, BlackwallGate, RegressionCase,
    TransactionOutcome,
};
use relic_core::ArithmeticOp;
use relic_lexicon::LexiconPayload;

#[test]
fn test_telemetry_passive_monitor_does_not_mutate_engram_wal() {
    let mut gate = BlackwallGate::with_genesis();
    assert_eq!(gate.telemetry_arena.mode, AnomalyGateMode::PassiveMonitor);

    let initial_version = gate.current_version();
    let initial_entries = gate.log_entries().len();

    // Enviar 50 puntos continuos de telemetría por el canal 7
    for step in 1..=50 {
        let val = 20.0 + (step as f64) * 0.1;
        let outcome = gate
            .commit_transaction_with_telemetry(&[], &[(7, val)], step)
            .expect("Transacción de telemetría válida");

        match outcome {
            TransactionOutcome::Committed { version, .. } => {
                assert_eq!(version, initial_version, "La versión del WAL no debe avanzar");
            }
            other => panic!("Esperaba Committed, obtuve: {:?}", other),
        }
    }

    // Aislamiento verificado: el WAL NO contiene eventos de telemetría
    assert_eq!(gate.current_version(), initial_version);
    assert_eq!(gate.log_entries().len(), initial_entries);

    // Auditoría fuera de banda contiene los 50 registros
    assert_eq!(gate.telemetry_arena.audit_log.len(), 50);
    for (i, record) in gate.telemetry_arena.audit_log.iter().enumerate() {
        assert_eq!(record.channel_id, 7);
        assert_eq!(record.timestamp, (i + 1) as u64);
        assert_eq!(record.mode, AnomalyGateMode::PassiveMonitor);
        assert!(record.value >= 20.0);
    }
}

#[test]
fn test_telemetry_rollback_on_oracle_or_regression_rejection() {
    let mut gate = BlackwallGate::with_genesis();

    // 1. Priming: Ingerir 20 puntos válidos en el canal 1
    for step in 1..=20 {
        let val = 10.0 + (step as f64) * 0.05;
        let outcome = gate
            .commit_transaction_with_telemetry(&[], &[(1, val)], step)
            .expect("Transacción de telemetría válida");
        assert!(matches!(outcome, TransactionOutcome::Committed { .. }));
    }

    assert_eq!(gate.telemetry_arena.audit_log.len(), 20);
    let h_before = gate.telemetry_arena.channels.get(&1).unwrap().esn.h;
    let norm_count_before = gate.telemetry_arena.channels.get(&1).unwrap().esn.norm.standardize(10.0);

    // 2. Proponer transacción combinada con violación de oráculo (redefinir palabra clave reservada "más")
    let bad_payload = LexiconPayload::op_alias("más", ArithmeticOp::Subtract);

    let outcome = gate
        .commit_transaction_with_telemetry(&[bad_payload], &[(1, 9999.0)], 21)
        .expect("Transacción evaluada");

    assert!(
        matches!(outcome, TransactionOutcome::RejectedByOracle { .. }),
        "Debe ser rechazada por el oráculo"
    );

    // 3. Verificar que el estado del detector y la auditoría permanecen intactos
    assert_eq!(gate.telemetry_arena.audit_log.len(), 20, "No debe haber registros adicionales en audit_log");
    let h_after = gate.telemetry_arena.channels.get(&1).unwrap().esn.h;
    assert_eq!(h_before, h_after, "El estado oculto h del reservorio debe permanecer inalterado tras rollback");
    let norm_count_after = gate.telemetry_arena.channels.get(&1).unwrap().esn.norm.standardize(10.0);
    assert_eq!(norm_count_before, norm_count_after, "La normalización rodante no debe haber ingerido el valor rechazado");

    // 4. Proponer transacción combinada con violación de regresión causal
    gate.add_regression_case(RegressionCase {
        id: "reg_embolatan".to_string(),
        query: "Si tengo 50 y me embolatan 15, ¿cuánto queda?".to_string(),
        expected_substring: "35".to_string(),
        expected_route: "deterministic_arithmetic".to_string(),
        expected_status: "VALID".to_string(),
    });

    // Fijar primero la semántica legítima de "embolatan"
    let commit_legit = gate.commit_transaction_with_telemetry(
        &[LexiconPayload::op_alias("embolatan", ArithmeticOp::Subtract)],
        &[],
        21,
    ).expect("Transacción de preparación");
    assert!(matches!(commit_legit, TransactionOutcome::Committed { .. }));

    let h_before_reg = gate.telemetry_arena.channels.get(&1).unwrap().esn.h;
    let audit_len_before_reg = gate.telemetry_arena.audit_log.len();

    // Intentar redefinir "embolatan" como suma (lo que rompe la regresión causal) con telemetría en canal 1
    let poison_payload = LexiconPayload::op_alias("embolatan", ArithmeticOp::Add);
    let outcome_reg = gate
        .commit_transaction_with_telemetry(&[poison_payload], &[(1, 8888.0)], 22)
        .expect("Transacción evaluada");

    match outcome_reg {
        TransactionOutcome::RejectedByRegression { ref failed_case, .. } => {
            assert_eq!(failed_case, "reg_embolatan");
        }
        other => panic!("Esperaba RejectedByRegression, obtuve: {:?}", other),
    }

    // El estado del canal y la auditoría siguen intactos tras el rechazo por regresión
    assert_eq!(gate.telemetry_arena.audit_log.len(), audit_len_before_reg);
    let h_after_reg = gate.telemetry_arena.channels.get(&1).unwrap().esn.h;
    assert_eq!(h_before_reg, h_after_reg);
}

#[test]
fn test_telemetry_channel_contention_rejected() {
    let mut gate = BlackwallGate::with_genesis();

    // Intentar enviar dos puntos para el mismo canal en el mismo lote
    let telemetry_batch = vec![(3, 10.5), (3, 11.2)];
    let err = gate
        .commit_transaction_with_telemetry(&[], &telemetry_batch, 1)
        .unwrap_err();

    assert_eq!(
        err,
        BlackwallError::ChannelContention { channel_id: 3 },
        "Debe rechazar contención intra-lote"
    );

    assert_eq!(gate.telemetry_arena.audit_log.len(), 0);
}

#[test]
fn test_telemetry_arena_exhaustion_returns_error_without_panic() {
    let mut gate = BlackwallGate::with_genesis();
    // Forzar capacidad máxima de 2 canales
    gate.telemetry_arena.max_channels = 2;

    // Canal 101 -> ok
    let r1 = gate.commit_transaction_with_telemetry(&[], &[(101, 1.0)], 1);
    assert!(r1.is_ok());

    // Canal 102 -> ok
    let r2 = gate.commit_transaction_with_telemetry(&[], &[(102, 2.0)], 2);
    assert!(r2.is_ok());

    // Canal 103 -> debe retornar ArenaExhausted
    let r3 = gate.commit_transaction_with_telemetry(&[], &[(103, 3.0)], 3);
    assert_eq!(
        r3.unwrap_err(),
        BlackwallError::ArenaExhausted { max: 2 },
        "Debe rechazar sin pánico al agotar la capacidad"
    );
}

#[test]
fn test_telemetry_enforcing_mode_rejects_anomalies_and_preserves_state() {
    let mut gate = BlackwallGate::with_genesis();
    gate.telemetry_arena.mode = AnomalyGateMode::Enforcing;

    // Enviar valores normales
    for step in 1..=10 {
        let outcome = gate
            .commit_transaction_with_telemetry(&[], &[(0, 50.0)], step)
            .expect("Paso normal debe pasar");
        assert!(matches!(outcome, TransactionOutcome::Committed { .. }));
    }

    assert_eq!(gate.telemetry_arena.audit_log.len(), 10);
    let h_clean = gate.telemetry_arena.channels.get(&0).unwrap().esn.h;

    // Inyectar un valor patológico (NaN)
    let outcome_nan = gate
        .commit_transaction_with_telemetry(&[], &[(0, f64::NAN)], 11)
        .expect("Evaluación debe ser procesada");

    match outcome_nan {
        TransactionOutcome::RejectedByAnomaly { channel_id, reason } => {
            assert_eq!(channel_id, 0);
            assert!(reason.contains("Violación"));
        }
        other => panic!("Esperaba RejectedByAnomaly, obtuve: {:?}", other),
    }

    // Verificar que el estado del detector NO fue envenenado por el NaN
    assert_eq!(gate.telemetry_arena.audit_log.len(), 10, "No debe registrarse en audit_log tras rechazo");
    let h_post_nan = gate.telemetry_arena.channels.get(&0).unwrap().esn.h;
    assert_eq!(h_clean, h_post_nan, "El reservorio no debe ser alterado por un valor rechazado");
}

#[test]
fn test_telemetry_combined_atomic_commit_with_graph_mutation() {
    let mut gate = BlackwallGate::with_genesis();

    // Registrar binding para canal 5 apuntando a la arista server_a -> active
    gate.telemetry_arena
        .register_channel(
            5,
            BindingTarget::Edge {
                source: "server_a".to_string(),
                target: "active".to_string(),
            },
        )
        .expect("Registro de canal exitoso");

    let payload = LexiconPayload::graph_fact_weighted("server_a", "state", "active", 0.95);

    let initial_version = gate.current_version();
    let outcome = gate
        .commit_transaction_with_telemetry(&[payload], &[(5, 42.0)], 100)
        .expect("Transacción combinada exitosa");

    match outcome {
        TransactionOutcome::Committed { version, .. } => {
            assert_eq!(version, initial_version + 1, "La mutación de grafo sí debe avanzar la versión");
        }
        other => panic!("Esperaba Committed, obtuve: {:?}", other),
    }

    // El grafo tiene la arista
    let neighbors = gate.retrieve_neighbors("server_a");
    assert!(neighbors.contains(&"active".to_string()));

    // La telemetría tiene el registro
    assert_eq!(gate.telemetry_arena.audit_log.len(), 1);
    assert_eq!(gate.telemetry_arena.audit_log[0].channel_id, 5);
    assert_eq!(gate.telemetry_arena.audit_log[0].value, 42.0);
}

#[test]
fn test_wal_byte_for_byte_identity_between_disabled_and_passive_monitor_with_mixed_traffic() {
    let mut gate_disabled = BlackwallGate::with_genesis();
    gate_disabled.telemetry_arena.mode = AnomalyGateMode::Disabled;

    let mut gate_passive = BlackwallGate::with_genesis();
    gate_passive.telemetry_arena.mode = AnomalyGateMode::PassiveMonitor;

    // Ejecutar 50 mutaciones idénticas con timestamps idénticos:
    // en gate_disabled solo van hechos de grafo
    // en gate_passive van los mismos hechos de grafo MÁS observaciones continuas de telemetría
    for i in 0..50 {
        let ts = 2_000_000_000 + (i as u64) * 10;
        let payload = LexiconPayload::graph_fact(
            format!("cluster_node_{i}"),
            "connects",
            format!("cluster_node_{}", (i + 1) % 50),
        );

        let out_dis = gate_disabled
            .commit_transaction_with_telemetry(std::slice::from_ref(&payload), &[], ts)
            .expect("Commit en Disabled");
        let out_pas = gate_passive
            .commit_transaction_with_telemetry(&[payload], &[(1, 100.0 + (i as f64) * 0.5)], ts)
            .expect("Commit en PassiveMonitor");

        assert_eq!(out_dis, out_pas, "El resultado de la transacción debe ser exactamente idéntico");
    }

    // 1. Versiones idénticas
    assert_eq!(gate_disabled.current_version(), gate_passive.current_version());
    assert_eq!(gate_disabled.current_version(), 13 + 50);

    // 2. Hash de cabecera idéntico (SHA-256 de la cadena de bloques del engrama)
    assert_eq!(gate_disabled.head_hash(), gate_passive.head_hash());

    // 3. Comparación BYTE A BYTE de la serialización engram.jsonl
    let wal_disabled_str = gate_disabled.to_jsonl_string();
    let wal_passive_str = gate_passive.to_jsonl_string();
    assert_eq!(
        wal_disabled_str.as_bytes(),
        wal_passive_str.as_bytes(),
        "El archivo WAL engram.jsonl debe ser 100% idéntico byte a byte"
    );

    // 4. Verificar que la telemetría sí se procesó en el canal fuera de banda
    assert_eq!(gate_disabled.telemetry_arena.audit_log.len(), 0);
    assert_eq!(gate_passive.telemetry_arena.audit_log.len(), 50);
}

#[test]
fn test_quarantine_pool_ephemeral_ram_and_overflow_handling() {
    let mut gate = BlackwallGate::with_genesis();
    gate.telemetry_arena.max_channels = 32;
    gate.telemetry_arena.max_quarantine_slots = 16;

    // Registrar 17 canales enlazados a aristas
    for ch in 1..=17 {
        gate.telemetry_arena
            .register_channel(
                ch,
                BindingTarget::Edge {
                    source: format!("node_{ch}"),
                    target: format!("node_{}", ch + 100),
                },
            )
            .expect("Registro de canal exitoso");
    }

    // Inyectar anomalía severa (f64::NAN) en los primeros 16 canales
    for ch in 1..=16 {
        let ts = 10_000 + ch as u64;
        gate.commit_transaction_with_telemetry(&[], &[(ch, f64::NAN)], ts)
            .expect("Transacción en PassiveMonitor exitosa");
    }

    // Verificar que los 16 están en el pool de cuarentena en RAM
    assert_eq!(gate.telemetry_arena.quarantine_pool.len(), 16);
    assert_eq!(gate.telemetry_arena.audit_log.len(), 16);
    for record in &gate.telemetry_arena.audit_log {
        assert!(!record.pool_overflow, "No debe haber overflow con 16 slots");
        assert!(record.quarantine_target.is_some());
    }

    // Intentar cuarentenar el canal 17 (debe desbordar el pool sin pánico ni crecimiento ilimitado)
    gate.commit_transaction_with_telemetry(&[], &[(17, f64::NAN)], 20_000)
        .expect("Transacción 17 exitosa");

    assert_eq!(
        gate.telemetry_arena.quarantine_pool.len(),
        16,
        "El pool de cuarentena debe permanecer acotado a 16 slots"
    );
    let last_record = gate.telemetry_arena.audit_log.back().unwrap();
    assert_eq!(last_record.channel_id, 17);
    assert!(last_record.pool_overflow, "El registro 17 debe marcar pool_overflow=true");

    // Verificar liberación y limpieza de cuarentena
    assert!(gate.telemetry_arena.release_quarantine(1));
    assert_eq!(gate.telemetry_arena.quarantine_pool.len(), 15);
    gate.telemetry_arena.clear_quarantine_pool();
    assert_eq!(gate.telemetry_arena.quarantine_pool.len(), 0);

    // Invariante de WAL: 0 mutaciones en disco
    assert_eq!(gate.current_version(), 13);
    println!("✓ Aislamiento estricto de QuarantinePool en RAM (capacidad acotada a 16) y control de desbordamiento certificados.");
}

#[test]
fn test_topological_impact_set_via_ms_bfs_in_memory() {
    let mut gate = BlackwallGate::with_genesis();

    // 1. Construir topología en el grafo:
    // sensor_A -> gateway_1 -> hub_core -> actuator_B
    // backup_path -> hub_core
    let facts = [
        LexiconPayload::graph_fact("sensor_A", "routes_to", "gateway_1"),
        LexiconPayload::graph_fact("gateway_1", "routes_to", "hub_core"),
        LexiconPayload::graph_fact("hub_core", "routes_to", "actuator_B"),
        LexiconPayload::graph_fact("backup_path", "routes_to", "hub_core"),
    ];

    gate.commit_transaction_with_telemetry(&facts, &[], 1_000)
        .expect("Construcción de topología exitosa");

    // 2. Registrar pares críticos de SLO
    // Par 1: ("sensor_A", "actuator_B") - Depende críticamente de ("gateway_1", "hub_core")
    // Par 2: ("backup_path", "actuator_B") - NO depende de ("gateway_1", "hub_core")
    gate.telemetry_arena.register_critical_pair("sensor_A", "actuator_B");
    gate.telemetry_arena.register_critical_pair("backup_path", "actuator_B");

    // Enlazar canal 42 a la arista crítica ("gateway_1", "hub_core")
    gate.telemetry_arena
        .register_channel(
            42,
            BindingTarget::Edge {
                source: "gateway_1".to_string(),
                target: "hub_core".to_string(),
            },
        )
        .expect("Enlace de canal exitoso");

    // Verificar que antes de la anomalía, el fast-path de MS-BFS está activo (overrides vacío)
    assert!(
        gate.active_view().episodic.edge_world_overrides.is_empty(),
        "El fast-path debe estar activo antes de la evaluación"
    );

    // 3. Priming: Ingerir 105 puntos normales en canal 42 para superar warmup (100 pasos)
    for step in 1..=105 {
        let val = 20.0 + (step as f64) * 0.05;
        gate.commit_transaction_with_telemetry(&[], &[(42, val)], step as u64)
            .expect("Priming exitoso");
    }
    assert_eq!(gate.telemetry_arena.quarantine_pool.len(), 0, "No debe haber cuarentenas durante el régimen normal");

    // 4. Inyectar anomalía severa en el canal 42 (999.0 supera holgadamente la cota histórica [20.0, 25.25])
    let ts = 2_000;
    gate.commit_transaction_with_telemetry(&[], &[(42, 999.0)], ts)
        .expect("Transacción de telemetría exitosa");

    // 5. Verificar auditoría y resultado del impacto topológico
    let record = gate.telemetry_arena.audit_log.back().unwrap();
    assert_eq!(record.channel_id, 42);
    assert!(record.score_relic >= 0.990 || record.range_violation);
    assert_eq!(
        record.quarantine_target,
        Some(BindingTarget::Edge {
            source: "gateway_1".to_string(),
            target: "hub_core".to_string(),
        })
    );

    // Debe identificar que SOLO ("sensor_A", "actuator_B") pierde conectividad
    assert_eq!(
        record.lost_reachability,
        vec![("sensor_A".to_string(), "actuator_B".to_string())],
        "MS-BFS debe detectar exactamente el par crítico afectado"
    );

    // 6. Verificar que el elemento está en cuarentena con el impacto registrado
    assert_eq!(gate.telemetry_arena.quarantine_pool.len(), 1);
    let q_item = &gate.telemetry_arena.quarantine_pool[0];
    assert_eq!(q_item.channel_id, 42);
    assert_eq!(
        q_item.lost_reachability,
        vec![("sensor_A".to_string(), "actuator_B".to_string())]
    );

    // 7. INVARIANTE CRÍTICO DE RENDIMIENTO:
    // La máscara temporal contrafactual fue eliminada inmediatamente;
    // edge_world_overrides DEBE estar vacío para no penalizar consultas normales.
    assert!(
        gate.active_view().episodic.edge_world_overrides.is_empty(),
        "edge_world_overrides DEBE limpiarse de inmediato para no penalizar el fast-path O(1) de MS-BFS"
    );

    println!("✓ Cálculo de conjunto de impacto topológico vía MS-BFS contrafactual y preservación de fast-path certificados.");
}
