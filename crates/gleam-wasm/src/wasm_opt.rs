use std::process::Command;

pub struct WasmOpt {
    binary: String,
}

impl Default for WasmOpt {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmOpt {
    pub fn new() -> Self {
        WasmOpt {
            binary: "wasm-opt".into(),
        }
    }

    pub fn is_available(&self) -> bool {
        Command::new(&self.binary).arg("--version").output().is_ok()
    }

    pub fn optimize(
        &self,
        input: &[u8],
        level: OptLevel,
        features: &[OptimizationFeature],
    ) -> Result<Vec<u8>, String> {
        if !self.is_available() {
            return Err(format!(
                "{} not found in PATH. Install Binaryen (https://github.com/WebAssembly/binaryen)",
                self.binary
            ));
        }

        let mut cmd = Command::new(&self.binary);
        cmd.arg("--input").arg("-");

        match level {
            OptLevel::O0 => {
                cmd.arg("-O0");
            }
            OptLevel::O1 => {
                cmd.arg("-O1");
            }
            OptLevel::O2 => {
                cmd.arg("-O2");
            }
            OptLevel::O3 => {
                cmd.arg("-O3");
            }
            OptLevel::Os => {
                cmd.arg("-Os");
            }
            OptLevel::Oz => {
                cmd.arg("-Oz");
            }
        }

        for feature in features {
            cmd.arg(match feature {
                OptimizationFeature::GcRefs => "--gcrefs",
                OptimizationFeature::Dce => "--dce",
                OptimizationFeature::Vacuum => "--vacuum",
                OptimizationFeature::Converge => "--converge",
                OptimizationFeature::RemoveUnusedModuleElements => {
                    "--remove-unused-module-elements"
                }
            });
        }

        cmd.arg("--output").arg("-");
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn {binary}: {e}", binary = self.binary))?;

        if let Some(ref mut stdin) = child.stdin {
            use std::io::Write;
            stdin
                .write_all(input)
                .map_err(|e| format!("write error: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("{binary} failed: {e}", binary = self.binary))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("{binary} failed: {stderr}", binary = self.binary));
        }

        Ok(output.stdout)
    }

    pub fn optimize_for_min_size(input: &[u8]) -> Result<Vec<u8>, String> {
        let opt = Self::new();
        opt.optimize(
            input,
            OptLevel::Os,
            &[
                OptimizationFeature::GcRefs,
                OptimizationFeature::Dce,
                OptimizationFeature::Vacuum,
                OptimizationFeature::Converge,
                OptimizationFeature::RemoveUnusedModuleElements,
            ],
        )
    }

    pub fn size_check(input: &[u8], max_bytes: usize) -> Result<bool, String> {
        match Self::optimize_for_min_size(input) {
            Ok(optimized) => Ok(optimized.len() <= max_bytes),
            Err(_) => {
                // If wasm-opt not available, check raw size
                Ok(input.len() <= max_bytes)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    O0,
    O1,
    O2,
    O3,
    Os,
    Oz,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationFeature {
    GcRefs,
    Dce,
    Vacuum,
    Converge,
    RemoveUnusedModuleElements,
}
