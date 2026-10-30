//! # UnsatCore and Local Constraint Repair Falsification Test Suite

use relic_core::repair::{
    EdgeOperatingBounds, EnvelopeStatus, LocalConstraintRepairer, RepairOutcome, StagedEnvelope,
    ViolationType,
};
use std::time::Instant;

#[test]
fn test_unsat_core_and_deterministic_repairer_four_scenarios() {
    let bounds = [
        EdgeOperatingBounds::new(0.0, 300.0, 50.0),
        EdgeOperatingBounds::new(0.0, 350.0, 60.0),
        EdgeOperatingBounds::new(0.0, 380.0, 80.0),
        EdgeOperatingBounds::new(0.0, 400.0, 70.0),
    ];
    let max_substation_budget = 1000.0f64;

    // Caso 1: Orden Limpia y Conforme (CertifiedAsIs)
    {
        let mut env = StagedEnvelope::new(101);
        env.add_op(0, 150.0, 140.0, 0).unwrap();
        env.add_op(1, 200.0, 190.0, 0).unwrap();
        env.add_op(2, 250.0, 240.0, 0).unwrap();
        env.add_op(3, 100.0, 95.0, 0).unwrap();

        let outcome = LocalConstraintRepairer::repair_or_veto(&mut env, &bounds, max_substation_budget);
        assert_eq!(outcome, RepairOutcome::CertifiedAsIs);
        assert_eq!(env.status, EnvelopeStatus::Certified);
        for op in env.ops_slice() {
            assert!(op.certified);
            assert!(!op.repaired);
        }
    }

    // Caso 2: Violación Paramétrica Reparable (RepairedInPlace)
    {
        let mut env = StagedEnvelope::new(202);
        env.add_op(0, 320.0, 200.0, 0).unwrap(); // Capacidad (300) y rampa (250) excedidas
        env.add_op(1, 340.0, 300.0, 0).unwrap();
        env.add_op(2, 370.0, 300.0, 0).unwrap();
        env.add_op(3, 300.0, 250.0, 0).unwrap(); // Suma = 1260 > 1000

        let outcome = LocalConstraintRepairer::repair_or_veto(&mut env, &bounds, max_substation_budget);
        match outcome {
            RepairOutcome::RepairedInPlace { core, repaired_ops_mask } => {
                assert!(core.count > 0);
                assert!(core.all_repairable());
                assert_ne!(repaired_ops_mask, 0);
                assert_eq!(env.status, EnvelopeStatus::Repaired);

                let ops = env.ops_slice();
                assert!(ops[0].requested_val <= 250.0);
                let total_post: f64 = ops.iter().map(|o| o.requested_val).sum();
                assert!(total_post <= max_substation_budget + 1e-6);
            }
            _ => panic!("Se esperaba RepairedInPlace"),
        }
    }

    // Caso 3: Arista en Cuarentena (VetoedIrreparable + JSON Stack)
    {
        let mut bounds_with_quarantine = bounds;
        bounds_with_quarantine[3] = EdgeOperatingBounds::quarantined();

        let mut env = StagedEnvelope::new(303);
        env.add_op(0, 100.0, 90.0, 0).unwrap();
        env.add_op(3, 50.0, 0.0, 0).unwrap();

        let outcome = LocalConstraintRepairer::repair_or_veto(&mut env, &bounds_with_quarantine, max_substation_budget);
        match outcome {
            RepairOutcome::VetoedIrreparable { core } => {
                assert_eq!(env.status, EnvelopeStatus::Vetoed);
                assert!(!core.all_repairable());
                let has_quarantine = core.violations.iter().any(|v| v.violation_type == ViolationType::EdgeQuarantined);
                assert!(has_quarantine);

                let mut json_buf = [0u8; 512];
                let written = LocalConstraintRepairer::format_unsat_json(&core, &mut json_buf);
                assert!(written > 0);
                let json_str = core::str::from_utf8(&json_buf[..written]).unwrap();
                assert!(json_str.contains("EDGE_QUARANTINED"));
                assert!(json_str.contains("\"edge\":3"));
            }
            _ => panic!("Se esperaba VetoedIrreparable"),
        }
    }

    // Caso 4: Parámetros Inválidos y Benchmark de Latencia
    {
        let mut nan_env = StagedEnvelope::new(404);
        nan_env.add_op(0, f64::NAN, 100.0, 0).unwrap();
        let nan_outcome = LocalConstraintRepairer::repair_or_veto(&mut nan_env, &bounds, max_substation_budget);
        match nan_outcome {
            RepairOutcome::VetoedIrreparable { core } => {
                assert_eq!(core.violations[0].violation_type, ViolationType::NonFiniteParameter);
            }
            _ => panic!("NaN debe vetar"),
        }

        const BENCH_ITERATIONS: usize = 10_000;
        let start = Instant::now();
        for i in 0..BENCH_ITERATIONS {
            let mut env = StagedEnvelope::new(i as u64);
            env.add_op(0, 250.0, 200.0, 0).unwrap();
            env.add_op(1, 320.0, 250.0, 0).unwrap();
            env.add_op(2, 420.0, 300.0, 0).unwrap();
            env.add_op(3, 210.0, 180.0, 0).unwrap();
            let res = LocalConstraintRepairer::repair_or_veto(&mut env, &bounds, max_substation_budget);
            std::hint::black_box(res);
        }
        let elapsed = start.elapsed();
        let avg_micros = (elapsed.as_nanos() as f64 / BENCH_ITERATIONS as f64) / 1000.0;
        assert!(avg_micros < 15.0, "Latencia promedio excede 15 µs: {:.3} µs", avg_micros);
    }
}
