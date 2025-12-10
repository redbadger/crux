build:
    cargo build

clean:
    cargo xtask --all clean

test:
    cargo insta test --review --test-runner nextest --all-features --lib

fix:
    cargo xtask --all format --fix

ci:
    cargo xtask --all ci
