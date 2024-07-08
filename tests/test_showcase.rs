mod common;
use common::assert_multiline_eq;
use escargot::{CargoBuild, CargoRun};
use std::{process::Command, str::from_utf8, sync::OnceLock};

fn get_built_showcase_example() -> Command {
    static ONCE: OnceLock<CargoRun> = OnceLock::new();
    ONCE.get_or_init(|| CargoBuild::new().example("showcase").run().unwrap())
        .command()
}

#[test]
fn test_showcase_example_no_args() {
    let mut command = get_built_showcase_example();
    let output = command.output().unwrap();

    let expected = &"
error: the following required arguments were not provided:
  --auth-url <solver_service.auth.url>
  --auth-retries <solver_service.auth.retries>
  --artifact-url <solver_service.artifact.url>
  --artifact-retries <solver_service.artifact.retries>
  --basic-solver-matrix-size <basic_solver.matrix_size>
  --basic-solver-branching-factor <basic_solver.branching_factor>
  --basic-solver-epsilon <basic_solver.epsilon>
  --high-priority-solver-matrix-size <high_priority_solver.matrix_size>
  --high-priority-solver-branching-factor <high_priority_solver.branching_factor>
  --high-priority-solver-epsilon <high_priority_solver.epsilon>
  --telemetry-url <telemetry.url>
  --telemetry-retries <telemetry.retries>

Usage: showcase --auth-url <solver_service.auth.url> --auth-retries <solver_service.auth.retries> --artifact-url <solver_service.artifact.url> --artifact-retries <solver_service.artifact.retries> --basic-solver-matrix-size <basic_solver.matrix_size> --basic-solver-branching-factor <basic_solver.branching_factor> --basic-solver-epsilon <basic_solver.epsilon> --high-priority-solver-matrix-size <high_priority_solver.matrix_size> --high-priority-solver-branching-factor <high_priority_solver.branching_factor> --high-priority-solver-epsilon <high_priority_solver.epsilon> --telemetry-url <telemetry.url> --telemetry-retries <telemetry.retries>

For more information, try '--help'.
"[1..];

    assert_multiline_eq!(from_utf8(&output.stderr).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn test_showcase_example_help() {
    let mut command = get_built_showcase_example();
    let output = command.args(["--help"]).output().unwrap();

    let expected = &"
Solves widget optimization problems on demand as a service

Usage: showcase [OPTIONS] --auth-url <solver_service.auth.url> --auth-retries <solver_service.auth.retries> --artifact-url <solver_service.artifact.url> --artifact-retries <solver_service.artifact.retries> --basic-solver-matrix-size <basic_solver.matrix_size> --basic-solver-branching-factor <basic_solver.branching_factor> --basic-solver-epsilon <basic_solver.epsilon> --high-priority-solver-matrix-size <high_priority_solver.matrix_size> --high-priority-solver-branching-factor <high_priority_solver.branching_factor> --high-priority-solver-epsilon <high_priority_solver.epsilon> --telemetry-url <telemetry.url> --telemetry-retries <telemetry.retries>

Options:
      --listen-addr <solver_service.listen_addr>
          Solver service: Listen address to bind to [env: MYCO_LISTEN_ADDR=] [default: 127.0.0.1:4040]
      --auth-url <solver_service.auth.url>
          Solver service: Auth service: Base URL [env: MYCO_AUTH_URL=]
      --auth-retries <solver_service.auth.retries>
          Solver service: Auth service: Number of retries [env: MYCO_AUTH_RETRIES=]
      --artifact-url <solver_service.artifact.url>
          Solver service: Artifact service: Base URL [env: MYCO_ARTIFACT_URL=]
      --artifact-retries <solver_service.artifact.retries>
          Solver service: Artifact service: Number of retries [env: MYCO_ARTIFACT_RETRIES=]
      --basic-solver-matrix-size <basic_solver.matrix_size>
          Basic solver: Size of matrix to use when solving [env: MYCO_BASIC_SOLVER_MATRIX_SIZE=]
      --basic-solver-branching-factor <basic_solver.branching_factor>
          Basic solver: Branching factor to use when exploring search tree [env: MYCO_BASIC_SOLVER_BRANCHING_FACTOR=]
      --basic-solver-epsilon <basic_solver.epsilon>
          Basic solver: Epsilon which controls when we decide that solutions have stopped improving significantly [env: MYCO_BASIC_SOLVER_EPSILON=]
      --basic-solver-deterministic-rng
          Basic solver: Whether to use a deterministic seeded rng [env: MYCO_BASIC_SOLVER_DETERMINISTIC_RNG=]
      --basic-solver-round-limit <basic_solver.round_limit>
          Basic solver: Maximum number of rounds before we stop the search [env: MYCO_BASIC_SOLVER_ROUND_LIMIT=]
      --high-priority-solver-matrix-size <high_priority_solver.matrix_size>
          High-priority solver: Size of matrix to use when solving [env: MYCO_HIGH_PRIORITY_SOLVER_MATRIX_SIZE=]
      --high-priority-solver-branching-factor <high_priority_solver.branching_factor>
          High-priority solver: Branching factor to use when exploring search tree [env: MYCO_HIGH_PRIORITY_SOLVER_BRANCHING_FACTOR=]
      --high-priority-solver-epsilon <high_priority_solver.epsilon>
          High-priority solver: Epsilon which controls when we decide that solutions have stopped improving significantly [env: MYCO_HIGH_PRIORITY_SOLVER_EPSILON=]
      --high-priority-solver-deterministic-rng
          High-priority solver: Whether to use a deterministic seeded rng [env: MYCO_HIGH_PRIORITY_SOLVER_DETERMINISTIC_RNG=]
      --high-priority-solver-round-limit <high_priority_solver.round_limit>
          High-priority solver: Maximum number of rounds before we stop the search [env: MYCO_HIGH_PRIORITY_SOLVER_ROUND_LIMIT=]
      --peer <peer_urls>
          Peers to which we can try to loadshed [env: MYCO_PEER_URLS=]
      --admin-listen-addr <admin_listen_addr>
          Admin listen address to bind to [env: MYCO_ADMIN_LISTEN_ADDR=] [default: 127.0.0.1:9090]
      --telemetry-url <telemetry.url>
          Telemetry endpoint: Base URL [env: MYCO_TELEMETRY_URL=]
      --telemetry-retries <telemetry.retries>
          Telemetry endpoint: Number of retries [env: MYCO_TELEMETRY_RETRIES=]
  -h, --help
          Print help
"[1..];

    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_showcase_example_success_args() {
    let mut command = get_built_showcase_example();
    let output = command
        .args([
            "--auth-url=https://example.com",
            "--auth-retries=7",
            "--artifact-url",
            "https://what.com",
            "--artifact-retries",
            "9",
            "--basic-solver-matrix-size=2000",
            "--basic-solver-branching-factor=3",
            "--basic-solver-epsilon",
            "-0.999",
            "--high-priority-solver-matrix-size=10000",
            "--high-priority-solver-branching-factor=4",
            "--high-priority-solver-epsilon",
            "-1.101",
            "--telemetry-url=https://far.scape",
            "--telemetry-retries=2",
            "--peer",
            "http://replica1.service.local",
            "--peer",
            "http://replica2.service.local",
        ])
        .output()
        .unwrap();

    let expected = &r#"
Config {
    solver_service: SolveServiceConfig {
        listen_addr: 127.0.0.1:4040,
        auth: HttpClientConfig {
            url: "https://example.com/",
            retries: 7,
        },
        artifact: HttpClientConfig {
            url: "https://what.com/",
            retries: 9,
        },
    },
    basic_solver: SolverConfig {
        matrix_size: 2000,
        branching_factor: 3,
        epsilon: -0.999,
        deterministic_rng: false,
        round_limit: None,
    },
    high_priority_solver: SolverConfig {
        matrix_size: 10000,
        branching_factor: 4,
        epsilon: -1.101,
        deterministic_rng: false,
        round_limit: None,
    },
    peer_urls: [
        "http://replica1.service.local/",
        "http://replica2.service.local/",
    ],
    admin_listen_addr: 127.0.0.1:9090,
    telemetry: HttpClientConfig {
        url: "https://far.scape/",
        retries: 2,
    },
}
"#[1..];

    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_showcase_example_success_env() {
    let mut command = get_built_showcase_example();
    let output = command
        .envs([
            ("MYCO_AUTH_URL", "https://example.com"),
            ("MYCO_AUTH_RETRIES", "7"),
            ("MYCO_ARTIFACT_URL", "https://what.com"),
            ("MYCO_ARTIFACT_RETRIES", "9"),
            ("MYCO_BASIC_SOLVER_MATRIX_SIZE", "2000"),
            ("MYCO_BASIC_SOLVER_BRANCHING_FACTOR", "3"),
            ("MYCO_BASIC_SOLVER_EPSILON", "0.001"),
            ("MYCO_HIGH_PRIORITY_SOLVER_MATRIX_SIZE", "10000"),
            ("MYCO_HIGH_PRIORITY_SOLVER_BRANCHING_FACTOR", "4"),
            ("MYCO_HIGH_PRIORITY_SOLVER_EPSILON", "-9.991"),
            ("MYCO_TELEMETRY_URL", "https://far.scape"),
            ("MYCO_TELEMETRY_RETRIES", "2"),
            ("MYCO_PEER_URLS", "http://replica1.service.local,http://replica2.service.local"),
        ])
        .output()
        .unwrap();

    let expected = &r#"
Config {
    solver_service: SolveServiceConfig {
        listen_addr: 127.0.0.1:4040,
        auth: HttpClientConfig {
            url: "https://example.com/",
            retries: 7,
        },
        artifact: HttpClientConfig {
            url: "https://what.com/",
            retries: 9,
        },
    },
    basic_solver: SolverConfig {
        matrix_size: 2000,
        branching_factor: 3,
        epsilon: 0.001,
        deterministic_rng: false,
        round_limit: None,
    },
    high_priority_solver: SolverConfig {
        matrix_size: 10000,
        branching_factor: 4,
        epsilon: -9.991,
        deterministic_rng: false,
        round_limit: None,
    },
    peer_urls: [
        "http://replica1.service.local/",
        "http://replica2.service.local/",
    ],
    admin_listen_addr: 127.0.0.1:9090,
    telemetry: HttpClientConfig {
        url: "https://far.scape/",
        retries: 2,
    },
}
"#[1..];

    assert_multiline_eq!(from_utf8(&output.stdout).unwrap(), &expected);
    assert_eq!(output.status.code(), Some(0));
}
