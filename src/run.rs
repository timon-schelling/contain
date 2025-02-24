use thiserror::Error;

use crate::config::Config;

#[derive(Error, Debug)]
pub enum VmError {

}

pub fn run_vm(config: Config) -> Result<(), VmError> {

    Ok(())
}
