use std::collections::{HashMap, HashSet};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum Modules {
    Arpa,
}

#[derive(Clone, Debug)]
pub enum ModuleManagerError {
    ModuleNotLoaded,
    ModuleAlreadyLoaded,
}

pub type ModuleManagerResult<T> = Result<T, ModuleManagerError>;

pub struct ModuleWrapper {
    pub handle: JoinHandle<()>,
    pub kill_switch: Sender<()>,
}

pub trait Module {
    fn load(self) -> ModuleWrapper;
}

pub struct ModuleManager {
    modules: Arc<Mutex<HashMap<Modules, ModuleWrapper>>>,
    pub ref_modules: Arc<Mutex<Vec<Modules>>>,
}

impl ModuleManager {
    pub fn new() -> Self {
        ModuleManager {
            modules: Arc::new(Mutex::new(HashMap::new())),
            ref_modules: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn load_module(
        &mut self,
        module_key: Modules,
        module: ModuleWrapper,
    ) -> ModuleManagerResult<()> {
        let mut modules = self.modules.lock().unwrap();
        if modules.contains_key(&module_key) {
            return Err(ModuleManagerError::ModuleAlreadyLoaded);
        };

        modules.insert(module_key.clone(), module);

        self.ref_modules.lock().unwrap().push(module_key);

        Ok(())
    }

    pub fn unload_module(&mut self, module_key: Modules) -> ModuleManagerResult<()> {
        let mut modules = self.modules.lock().unwrap();

        if let Some(module) = modules.remove(&module_key) {
            module.kill_switch.send(()).unwrap();
            module.handle.join().unwrap();

            let mut ref_modules = self.ref_modules.lock().unwrap();
            if let Some(index) = ref_modules.iter().position(|m| *m == module_key) {
                ref_modules.remove(index);
            }

            Ok(())
        } else {
            Err(ModuleManagerError::ModuleNotLoaded)
        }
    }

    pub fn main(&mut self) -> () {
        loop {
            self.modules
                .lock()
                .unwrap()
                .retain(|_, val| !val.handle.is_finished());

            let mut ref_modules = self.ref_modules.lock().unwrap();
            ref_modules.retain(|module_key| self.modules.lock().unwrap().contains_key(&module_key));

            std::thread::sleep(Duration::new(1, 0));
        }
    }
}
