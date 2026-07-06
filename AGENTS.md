## Build & Run

**Setup**:
```bash
# Install dependencies
npm install
. $HOME/.cargo/env  # Source Rust environment

# Ensure C compiler is available (Rust dependencies need cc/gcc)
# Container: apt-get install -y gcc g++ make pkg-config
```

**Build**:
- Frontend: `npm run build` (transpiles TypeScript + Tailwind)
- Rust: `cargo build -p momentum` in src-tauri/
- Full app: `npm run tauri build` (requires C compiler)

**Typecheck**:
```bash
npx tsc --noEmit        # Frontend type checking
```

**Development**:
```bash
npm run dev             # Start Vite dev server (port 5173)
# Then in another terminal:
cargo tauri dev         # Launch dev app with hot reload
```

## Validation

Run these after implementing to get immediate feedback:

- Tests: `npm test` (Vitest - not yet configured)
- Typecheck: `npx tsc --noEmit`
- Lint: `npm run lint` (not yet configured)

## Operational Notes

**Build Environment Issues**:
- Container lacks C compiler (gcc/g++). Rust crates with C dependencies (rusqlite, libc, serde-core) fail to compile without it.
- Solution: Install build-essential in container, or switch to pure-Rust alternatives (e.g., sqlx over rusqlite).

**Project Structure**:
- Frontend: `/src/` (React + TypeScript)
- Backend: `/src-tauri/src/` (Rust, Tauri commands)
- Config: `tauri.conf.json`, `vite.config.ts`, `tsconfig.json`
- Specs: `/specs/01-05_*.md` (frozen API contracts)

**Codebase Patterns**:
- Error handling: AppError enum with 8 variants (Database, SimpleFin, Llm, Validation, Config, Internal, Keychain, NotFound)
- Commands: Async for I/O (SimpleFIN, metrics), sync for mutations
- DTO serialization: serde-compatible (Rust ↔ JSON ↔ TypeScript)
- Logging: Rust uses `tracing` crate; React logging not yet configured