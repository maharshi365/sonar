import { copyFileSync, existsSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const profile = process.argv[2]
if (profile !== "debug" && profile !== "release") {
  throw new Error("Expected native build profile: debug or release")
}

const root = join(dirname(fileURLToPath(import.meta.url)), "..")
const addonDir = join(root, "src", "crates", "sonar-core")
const targetDir = join(addonDir, "target", profile)

let source
let target

switch (process.platform) {
  case "win32": {
    source = join(targetDir, "sonar_core.dll")
    target = join(addonDir, `sonar-core.win32-${process.arch}-msvc.node`)
    break
  }
  case "darwin": {
    source = join(targetDir, "libsonar_core.dylib")
    target = join(addonDir, `sonar-core.darwin-${process.arch}.node`)
    break
  }
  case "linux": {
    const libc = process.report?.getReport().header.glibcVersionRuntime
      ? "gnu"
      : "musl"
    source = join(targetDir, "libsonar_core.so")
    target = join(addonDir, `sonar-core.linux-${process.arch}-${libc}.node`)
    break
  }
  default:
    throw new Error(`Unsupported native addon platform: ${process.platform}`)
}

if (!existsSync(source)) {
  throw new Error(`Native build output not found: ${source}`)
}

copyFileSync(source, target)
console.log(`Staged native addon: ${target}`)
