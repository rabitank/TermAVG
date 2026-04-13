// path_resolver.rs - 独立的路径工具模块
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{env, fs};

// 单例模式，全局只初始化一次
static PATH_RESOLVER: OnceLock<PathResolver> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct PathResolver {
    /// 基础目录（根据环境自动确定）
    base_dir: PathBuf,
    /// 当前环境
    environment: Environment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Production,
}

impl Environment {
    pub fn is_dev(&self) -> bool {
        matches!(self, Environment::Development)
    }

    pub fn is_prod(&self) -> bool {
        matches!(self, Environment::Production)
    }
}

impl PathResolver {
    /// 初始化全局解析器（应在程序启动时调用一次）
    pub fn global_init() -> &'static Self {
        PATH_RESOLVER.get_or_init(|| Self::new().expect("初始化路径解析器失败"))
    }

    /// 获取全局解析器实例
    pub fn global() -> &'static Self {
        PATH_RESOLVER
            .get()
            .expect("路径解析器未初始化，请先调用 global_init()")
    }

    /// 创建新的解析器（自动检测环境）
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let exe_path = env::current_exe()?;
        let exe_dir = exe_path
            .parent()
            .ok_or("无法获取可执行文件目录")?
            .to_path_buf();

        // 检测环境
        let (environment, base_dir) = Self::detect_environment(&exe_dir)?;

        Ok(Self {
            base_dir,
            environment,
        })
    }

    /// 手动指定环境的构造函数
    pub fn with_environment(
        env: Environment,
        custom_base: Option<PathBuf>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let exe_path = env::current_exe()?;
        let exe_dir = exe_path
            .parent()
            .ok_or("无法获取可执行文件目录")?
            .to_path_buf();

        let base_dir = match (env, custom_base) {
            (Environment::Development, Some(path)) => path,
            (Environment::Development, None) => Self::find_project_root(&exe_dir)?,
            (Environment::Production, Some(path)) => path,
            (Environment::Production, None) => exe_dir,
        };

        Ok(Self {
            base_dir,
            environment: env,
        })
    }

    // 检测环境并确定基础目录
    fn detect_environment(
        exe_dir: &Path,
    ) -> Result<(Environment, PathBuf), Box<dyn std::error::Error>> {
        let exe_path_str = exe_dir.to_string_lossy();

        // 判断是否在开发环境（通过常见的开发目录特征）
        let is_development = exe_path_str.contains("target/debug")
            || exe_path_str.contains("target/release")
            || exe_path_str.contains("target") && exe_dir.join("Cargo.toml").exists()
            || env::var("CARGO_MANIFEST_DIR").is_ok(); // Cargo环境变量

        if is_development {
            // 开发环境：查找项目根目录
            let project_root = Self::find_project_root(exe_dir)?;
            Ok((Environment::Development, project_root))
        } else {
            // 生产环境：使用可执行文件所在目录
            Ok((Environment::Production, exe_dir.to_path_buf()))
        }
    }

    // 查找项目根目录（向上找直到找到Cargo.toml）
    fn find_project_root(start_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let mut current = Some(start_dir);

        while let Some(dir) = current {
            // 检查Cargo.toml
            if dir.join("Cargo.toml").exists() {
                return Ok(dir.to_path_buf());
            }

            // 检查其他项目标记（可选）
            if dir.join(".git").exists() || dir.join(".project").exists() {
                return Ok(dir.to_path_buf());
            }

            current = dir.parent();
        }

        // 没找到就回退到可执行文件目录
        Ok(start_dir.to_path_buf())
    }

    /// 获取当前环境
    pub fn environment(&self) -> Environment {
        self.environment
    }

    /// 获取基础目录
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// 解析路径（相对于基础目录）
    pub fn resolve(&self, relative: impl AsRef<Path>) -> PathBuf {
        let path = relative.as_ref();
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        }
    }

    /// 解析多个路径组件
    pub fn resolve_components(&self, components: &[&str]) -> PathBuf {
        let mut path = self.base_dir.clone();
        for component in components {
            path.push(component);
        }
        path
    }

    /// 获取相对于基础目录的路径（用于调试）
    pub fn relative_to_base(&self, absolute: &Path) -> Option<PathBuf> {
        absolute
            .strip_prefix(&self.base_dir)
            .ok()
            .map(|p| p.to_path_buf())
    }

    /// 创建目录（如果不存在）
    pub fn ensure_dir(&self, dir: impl AsRef<Path>) -> Result<PathBuf, std::io::Error> {
        let full_path = self.resolve(dir);
        if !full_path.exists() {
            std::fs::create_dir_all(&full_path)?;
        }
        Ok(full_path)
    }

    /// 确保文件存在，如果不存在则创建父目录和空文件
    pub fn ensure_file(&self, file: impl AsRef<Path>) -> Result<PathBuf, std::io::Error> {
        let full_path = self.resolve(file);

        // 确保父目录存在
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // 如果文件不存在，创建空文件
        if !full_path.exists() {
            fs::write(&full_path, "")?;
        }

        Ok(full_path)
    }

    /// 检查文件是否存在
    pub fn file_exists(&self, file: impl AsRef<Path>) -> bool {
        self.resolve(file).exists()
    }

    /// 删除文件（如果存在）
    pub fn remove_file(&self, file: impl AsRef<Path>) -> Result<bool, std::io::Error> {
        let full_path = self.resolve(file);
        if full_path.exists() {
            fs::remove_file(full_path)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    /// 确保文件存在，并提供默认内容（如果文件不存在）
    pub fn ensure_file_with_content(
        &self,
        file: impl AsRef<Path>,
        default_content: &str,
    ) -> Result<PathBuf, std::io::Error> {
        let full_path = self.resolve(file);

        // 确保父目录存在
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // 如果文件不存在，写入默认内容
        if !full_path.exists() {
            fs::write(&full_path, default_content)?;
        }

        Ok(full_path)
    }

    /// 确保文件存在，并提供默认内容生成器（异步或复杂内容）
    pub fn ensure_file_with<F>(
        &self,
        file: impl AsRef<Path>,
        creator: F,
    ) -> Result<PathBuf, std::io::Error>
    where
        F: FnOnce() -> Result<Vec<u8>, std::io::Error>,
    {
        let full_path = self.resolve(file);

        // 确保父目录存在
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // 如果文件不存在，使用creator生成内容
        if !full_path.exists() {
            let content = creator()?;
            fs::write(&full_path, content)?;
        }

        Ok(full_path)
    }
}

// 便捷函数 - 不需要先获取解析器实例

/// 快速解析路径（使用全局解析器）
pub fn path(relative: impl AsRef<Path>) -> PathBuf {
    PathResolver::global().resolve(relative)
}

/// 快速获取基础目录
pub fn base() -> &'static Path {
    PathResolver::global().base_dir()
}

/// 快速判断当前环境
pub fn is_development() -> bool {
    PathResolver::global().environment().is_dev()
}

/// 快速获取环境
pub fn environment() -> Environment {
    PathResolver::global().environment()
}
/// 快速确保文件存在
pub fn ensure_file(file: impl AsRef<Path>) -> Result<PathBuf, std::io::Error> {
    PathResolver::global().ensure_file(file)
}

pub fn ensure_dir(file: impl AsRef<Path>) -> Result<PathBuf, std::io::Error> {
    PathResolver::global().ensure_dir(file)
}
/// 快速确保文件存在（带默认内容）
pub fn ensure_file_with_content(
    file: impl AsRef<Path>,
    content: &str,
) -> Result<PathBuf, std::io::Error> {
    PathResolver::global().ensure_file_with_content(file, content)
}

