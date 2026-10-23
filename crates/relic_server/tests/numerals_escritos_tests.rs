//! Numerales escritos >15 y veinti-serie: regresión para la extensión de normalize.rs.
//! No toca suites congeladas; solo agrega casos nuevos. Menos de 180 líneas.

use relic_server::engine::execute_query;
use relic_server::state::ServerState;

fn contiene(texto_in: &str, aguja: &str) -> bool {
    let state = ServerState::new();
    let res = execute_query(texto_in, &state);
    let ok = res.text.contains(aguja) && res.route == "deterministic_arithmetic";
    if !ok {
        eprintln!("FALLO: input='{}' -> ruta={} texto='{}' (esperaba aguja '{}')", texto_in, res.route, res.text, aguja);
    }
    ok
}

#[test]
fn serie_veinti() {
    assert!(contiene("veintitrés más cuatro", "27"));
    assert!(contiene("veintitres mas cuatro", "27"));
    assert!(contiene("veintidós menos veintiocho", "-6"));
}

#[test]
fn serie_dieciseis_a_19() {
    assert!(contiene("dieciséis más dieciocho", "34"));
    assert!(contiene("dieciseis mas dieciocho", "34"));
    assert!(contiene("diecinueve menos diecisiete", "2"));
}

#[test]
fn decadas_simples() {
    assert!(contiene("treinta más doce", "42"));
    assert!(contiene("cuarenta menos siete", "33"));
    assert!(contiene("cincuenta por dos", "100"));
}

#[test]
fn aditivo_y_imperativo_con_numerales() {
    assert!(contiene("suma veinte e once", "31"));
    assert!(contiene("tengo veinte lápices y regalo doce", "8"));
}

#[test]
fn congelado_no_retrocede() {
    assert!(contiene("dos más dos", "4"));
    assert!(contiene("cuánto es 5 - 9", "-4"));
    assert!(contiene("resta 3 y 10", "-7"));
}
