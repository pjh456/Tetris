const path = require("path");
const fs = require("fs");
const { spawnSync } = require("child_process");

// ===== 工具 =====
function resolveRoot() {
    const cwd = process.cwd();
    return path.basename(cwd) === "scripts"
        ? path.resolve(cwd, "..")
        : cwd;
}

function check(cmd) {
    const isWin = process.platform === "win32";
    const checker = isWin ? "where" : "which";

    const res = spawnSync(checker, [cmd], { stdio: "ignore" });

    if (res.status !== 0) {
        console.error(`❌ Missing command in PATH: ${cmd}`);
        process.exit(1);
    }
}

function run(cmd, args, cwd) {
    console.log(`>> ${cmd} ${args.join(" ")}`);
    const res = spawnSync(cmd, args, {
        stdio: "inherit",
        cwd,
        shell: true,
    });

    if (res.status !== 0) {
        console.error(`❌ Command failed: ${cmd}`);
        process.exit(res.status);
    }
}

// ===== 主逻辑 =====
const ROOT = resolveRoot();
const ENGINE = path.join(ROOT, "engine");
const BUILD = path.join(ENGINE, "build");

// Debug / Release
const BUILD_TYPE = process.argv[2] || "Release";

function cleanBuild() {
    if (fs.existsSync(BUILD)) {
        console.log(">> cleaning build directory");
        fs.rmSync(BUILD, { recursive: true, force: true });
    }
    fs.mkdirSync(BUILD, { recursive: true });
}

function main() {
    check("cmake");

    cleanBuild();

    // configure
    run(
        "cmake",
        ["..", `-DCMAKE_BUILD_TYPE=${BUILD_TYPE}`],
        BUILD
    );

    // build（并行）
    run(
        "cmake",
        ["--build", ".", "--parallel"],
        BUILD
    );
}

main();