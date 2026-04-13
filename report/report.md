运行方式
cargo run --manifest-path app-helloworld/exercise/Cargo.toml
cargo run --manifest-path app-collections/exercise/Cargo.toml
cargo run --manifest-path app-msgqueue/exercise/test/Cargo.toml

loadapp exercise
cargo run --manifest-path app-loadapp/exercise/Cargo.toml

runlinuxapp 验证
# 先做代码编译验证（riscv64）
cd app-runlinuxapp && AX_CONFIG_PATH=/home/uibn/os/biglab-b/tg-arceos-tutorial/app-runlinuxapp/configs/riscv64.toml cargo check --features axstd --target riscv64gc-unknown-none-elf

# 端到端运行（需要 musl 交叉编译器，如 riscv64-linux-musl-gcc）
cd app-runlinuxapp && cargo xtask run