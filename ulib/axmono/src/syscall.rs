use core::ffi::{CStr, c_char};

use axhal::arch::TrapFrame;
use axhal::trap::{SYSCALL, register_trap_handler};
use axtask::current;
use syscalls::Sysno;
use axsyscall::sys_handler_def;
use crate::{
    ctypes::{CloneFlags, WaitStatus},
    task,
};
#[register_trap_handler(SYSCALL)]
fn handle_syscall(tf: &TrapFrame, syscall_num: usize) -> Option<isize> {
    let args = [
        tf.arg0(),
        tf.arg1(),
        tf.arg2(),
        tf.arg3(),
        tf.arg4(),
        tf.arg5(),
    ];

    let sys_id = Sysno::from(syscall_num as u32); //检查id与测例是否适配

    let ret = match sys_id {
        Sysno::clone => {
            let curr = current();
            let clone_flags = CloneFlags::from_bits(args[0] as u32);
            if clone_flags.is_none() {
                error!("Invalid clone flags: {:x}", args[0]);
                axtask::exit(-1); // FIXME: return error code
            }
            let clone_flags = clone_flags.unwrap();
            let sp = args[1];

            let child_task = task::clone_task(
                curr.as_task_ref().clone(),
                if (sp != 0) { Some(sp) } else { None },
                clone_flags,
                true,
            )
            .unwrap();
            axtask::spawn_task_by_ref(child_task.clone());
            Some(child_task.id().as_u64() as isize)
        }
        Sysno::wait4 => {
            let curr = current();
            let mut result = None;
            while let wait_result = task::wait_pid(
                curr.as_task_ref().clone(),
                args[0] as i32,
                args[1] as *mut i32,
            ) {
                let r = match wait_result {
                    Ok(pid) => {
                        result = Some(pid as isize);
                        break;
                    }
                    Err(WaitStatus::NotExist) => {
                        result = Some(0);
                        break;
                    }
                    Err(e) => {
                        debug!("wait4: {:?}, keep waiting...", e);
                    }
                };
            }
            result
        }
        Sysno::execve => {
            let program_name = unsafe { CStr::from_ptr((args[0] as *const u8).into()) };
            // FIXME: drop curr ref?
            match task::exec_current(program_name.to_str().expect("cannot convert").into()) {
                Ok(()) => {
                    unreachable!("Successful execve should not reach here");
                }
                Err(_) => Some(-1),
            }
        }
        _ => None,
    };
    ret
}

