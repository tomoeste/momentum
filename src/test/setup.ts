import { vi } from 'vitest'

// Mock tauri API if needed
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))
