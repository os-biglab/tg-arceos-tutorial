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