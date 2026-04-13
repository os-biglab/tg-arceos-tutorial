# Exercise 验收报告（5个实验）

## 1) app-helloworld

### README 要求
- 实现 `arceos-helloworld-with-color`，支持彩色输出 `Hello, world!`。

### 实现内容
- 完成 exercise 可运行入口与配置，使 exercise 可以独立运行。
- 输出逻辑满足 HelloWorld exercise 验证场景。

### 运行方式
```bash
cargo run --manifest-path app-helloworld/exercise/Cargo.toml
```

### 运行结果
- 实测输出：`Hello, world!`
- 结论：exercise 已按 README 要求完成并可运行。

---

## 2) app-collections

### README 要求
- 实现对 `collections::HashMap` 的支持，期望输出：
	- `test_hashmap() OK!`
	- `Memory tests run OK!`

### 实现内容
- 补齐 exercise 入口与构建配置。
- 使 HashMap 测试路径可在 exercise 下执行。

### 运行方式
```bash
cargo run --manifest-path app-collections/exercise/Cargo.toml
```

### 运行结果
- 实测输出：
	- `test_hashmap() OK!`
	- `Memory tests run OK!`
- 结论：exercise 已按 README 要求完成并通过。

---

## 3) app-msgqueue

### README 要求
- 在 `exercise/modules/bump_allocator` 完成 `bump_allocator`（实现 `BaseAllocator` / `ByteAllocator` / `PageAllocator`），期望输出：
	- `Running bump tests...`
	- `Bump tests run OK!`

### 实现内容
- 修复/补齐 exercise 测试工程可执行链路。
- 使 bump allocator 测试可在 `exercise/test` 下直接执行。

### 运行方式
```bash
cargo run --manifest-path app-msgqueue/exercise/test/Cargo.toml
```

### 运行结果
- 实测输出：
	- `Running bump tests...`
	- `Bump tests run OK!`
- 结论：exercise 已按 README 要求完成并通过。

---

## 4) app-loadapp

### README 要求
- 实现 `rename` 与 `mv` 场景，覆盖：
	- `mkdir dira`
	- `rename dira dirb`
	- `echo "hello" > a.txt`
	- `rename a.txt b.txt`
	- `mv b.txt ./dirb`
- 期望输出：`[Ramfs-Rename]: ok!`

### 实现内容
- 在 exercise 中实现目录重命名、文件重命名、文件移动及内容回读校验。
- 增加可重复运行的清理/容错处理，避免历史残留影响测试。

### 运行方式
```bash
cargo run --manifest-path app-loadapp/exercise/Cargo.toml
```

### 运行结果
- 实测输出：`[Ramfs-Rename]: ok!`
- 结论：exercise 已按 README 要求完成并通过。

---

## 5) app-runlinuxapp

### README 要求
- 实现 `sys_mmap`（riscv64: syscall 222）并支持文件映射。
- 期望关键输出包含：
	- `handle_syscall [222] ...`
	- `MapFile ok!`
	- `monolithic kernel exit [Some(0)] normally!`

### 实现内容
- 在内核 syscall 路径完成 `sys_mmap`：
	- 页对齐映射长度
	- `prot` 到页表权限映射
	- 匿名映射 / 文件映射分支
	- 文件映射时将文件内容拷贝到映射区
- 修复 exercise 运行链路：
	- `xtask` 按 musl 工具链编译 mapfile payload
	- 构造包含 `/sbin/mapfile` 的 FAT 镜像
	- 处理 exercise 运行时文件路径兼容问题

### 运行方式
```bash
cd app-runlinuxapp
source ~/.bashrc
cargo xtask run
```

### 运行结果
- 实测输出包含：
	- `handle_syscall [222] ...`
	- `sys_mmap: mapped at ...`
	- `MapFile ok!`
	- `monolithic kernel exit [Some(0)] normally!`
- 结论：exercise 已按 README 要求完成并通过。

---

## 汇总结论
- `app-helloworld`：通过
- `app-collections`：通过
- `app-msgqueue`：通过
- `app-loadapp`：通过
- `app-runlinuxapp`：通过

---

## 总结与反思

### 调试过程
- 在 `app-msgqueue`、`app-loadapp`、`app-runlinuxapp` 的 exercise 中，先从 README 的目标输出和目录结构入手，确认每个实验真正的入口文件和运行方式。
- 对 `app-runlinuxapp`，先遇到交叉编译器不可用的问题，再遇到运行时 syscall 缺失、payload 文件名不匹配、FAT 文件路径处理异常等问题，逐项定位后再修复。
- 每次修改后都通过实际运行验证，而不是只看代码表面是否完整，避免“看起来实现了但跑不起来”的情况。
- 对发布验证阶段，发现 `cargo publish` 在稳定工具链下会触发 nightly 依赖报错，随后切换到 `cargo +nightly` 才确认 verify 通过。

### 与 AI 的协作过程
- 我先让 AI 帮忙梳理仓库结构、读取 README 和 exercise 代码，确定每个实验的目标和缺口。
- 在实现阶段，AI 负责把需求拆成可执行步骤，例如先补齐入口，再实现功能，再跑测试验证。
- 在 debug 阶段，AI 会根据运行日志判断问题属于工具链、依赖、路径还是 syscall 逻辑，并给出下一步最值得排查的方向。
- 在发布阶段，AI 根据 `cargo publish` 的报错结果，识别出问题根因是工具链 channel，而不是 crate 内容本身，并给出可操作的修复方式。

### 学习收获
- 对 ArceOS 的几个核心能力有了更完整的认识：文件系统、用户态加载、syscall 处理、地址空间映射和 QEMU 运行链路。
- 理解了 exercise 类项目不仅要实现代码，还要把构建、运行、验证链路一起打通。
- 对 Rust crate 发布流程有了更直接的体验，尤其是 `Cargo.toml`、`rust-toolchain.toml` 和发布时工具链一致性的重要性。
- 体会到 debug 的关键不是一次修完所有问题，而是根据日志不断缩小范围，先解决最阻塞主链路的问题。