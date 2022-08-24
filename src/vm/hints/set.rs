use crate::serde::deserialize_program::ApTracking;
use crate::types::relocatable::MaybeRelocatable;
use crate::vm::errors::vm_errors::VirtualMachineError;
use crate::{bigint, vm::vm_core::VMProxy};
use num_bigint::BigInt;
use num_traits::{ToPrimitive, Zero};
use std::collections::HashMap;

use super::hint_utils::{
    get_integer_from_var_name, get_ptr_from_var_name, insert_value_from_var_name,
};

pub fn set_add(
    vm_proxy: &mut VMProxy,
    ids: &HashMap<String, usize>,
    hint_ap_tracking: Option<&ApTracking>,
) -> Result<(), VirtualMachineError> {
    let set_ptr = get_ptr_from_var_name("set_ptr", ids, vm_proxy, hint_ap_tracking)?;
    let elm_size = get_integer_from_var_name("elm_size", ids, vm_proxy, hint_ap_tracking)?
        .to_usize()
        .ok_or(VirtualMachineError::BigintToUsizeFail)?;
    let elm_ptr = get_ptr_from_var_name("elm_ptr", ids, vm_proxy, hint_ap_tracking)?;
    let set_end_ptr = get_ptr_from_var_name("set_end_ptr", ids, vm_proxy, hint_ap_tracking)?;

    if elm_size.is_zero() {
        return Err(VirtualMachineError::ValueNotPositive(bigint!(elm_size)));
    }
    let elm = vm_proxy
        .memory
        .get_range(&MaybeRelocatable::from(elm_ptr), elm_size)
        .map_err(VirtualMachineError::MemoryError)?;

    if set_ptr > set_end_ptr {
        return Err(VirtualMachineError::InvalidSetRange(
            MaybeRelocatable::from(set_ptr),
            MaybeRelocatable::from(set_end_ptr),
        ));
    }

    let range_limit = set_end_ptr.sub_rel(&set_ptr)?;

    for i in (0..range_limit).step_by(elm_size) {
        let set_iter = vm_proxy
            .memory
            .get_range(
                &MaybeRelocatable::from(set_ptr.clone() + i as usize),
                elm_size,
            )
            .map_err(VirtualMachineError::MemoryError)?;

        if set_iter == elm {
            insert_value_from_var_name(
                "index",
                bigint!(i / elm_size),
                ids,
                vm_proxy,
                hint_ap_tracking,
            )?;
            return insert_value_from_var_name(
                "is_elm_in_set",
                bigint!(1),
                ids,
                vm_proxy,
                hint_ap_tracking,
            );
        }
    }
    insert_value_from_var_name("is_elm_in_set", bigint!(0), ids, vm_proxy, hint_ap_tracking)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::exec_scope::get_exec_scopes_proxy;
    use crate::types::exec_scope::ExecutionScopes;
    use crate::types::hint_executor::HintExecutor;
    use crate::types::instruction::Register;
    use crate::utils::test_utils::*;
    use crate::vm::hints::execute_hint::{get_vm_proxy, BuiltinHintExecutor, HintReference};
    use crate::vm::runners::builtin_runner::RangeCheckBuiltinRunner;
    use crate::vm::vm_core::VirtualMachine;
    use num_bigint::Sign;

    static HINT_EXECUTOR: BuiltinHintExecutor = BuiltinHintExecutor {};

    const HINT_CODE: &str = "assert ids.elm_size > 0\nassert ids.set_ptr <= ids.set_end_ptr\nelm_list = memory.get_range(ids.elm_ptr, ids.elm_size)\nfor i in range(0, ids.set_end_ptr - ids.set_ptr, ids.elm_size):\n    if memory.get_range(ids.set_ptr + i, ids.elm_size) == elm_list:\n        ids.index = i // ids.elm_size\n        ids.is_elm_in_set = 1\n        break\nelse:\n    ids.is_elm_in_set = 0";

    fn init_vm_ids(
        set_ptr: Option<&MaybeRelocatable>,
        elm_size: Option<&MaybeRelocatable>,
        elm_a: Option<&MaybeRelocatable>,
        elm_b: Option<&MaybeRelocatable>,
    ) -> (VirtualMachine, HashMap<String, usize>) {
        let mut vm = vm_with_range_check!();

        for _ in 0..4 {
            vm.segments.add(&mut vm.memory, None);
        }

        vm.run_context.fp = 5;

        let set_ptr_default = MaybeRelocatable::from((2, 0));
        let elm_size_default = MaybeRelocatable::from(bigint!(2));
        let elm_a_default = MaybeRelocatable::from(bigint!(2));
        let elm_b_default = MaybeRelocatable::from(bigint!(3));

        vm.memory
            .insert(
                &MaybeRelocatable::from((1, 2)),
                set_ptr.unwrap_or(&set_ptr_default),
            )
            .expect("Unexpected memory insert fail");
        vm.memory
            .insert(
                &MaybeRelocatable::from((1, 3)),
                elm_size.unwrap_or(&elm_size_default),
            )
            .expect("Unexpected memory insert fail");
        vm.memory
            .insert(
                &MaybeRelocatable::from((1, 4)),
                &MaybeRelocatable::from((3, 0)),
            )
            .expect("Unexpected memory insert fail");
        vm.memory
            .insert(
                &MaybeRelocatable::from((1, 5)),
                &MaybeRelocatable::from((2, 2)),
            )
            .expect("Unexpected memory insert fail");
        vm.memory
            .insert(
                &MaybeRelocatable::from((2, 0)),
                &MaybeRelocatable::from(bigint!(1)),
            )
            .expect("Unexpected memory insert fail");
        vm.memory
            .insert(
                &MaybeRelocatable::from((2, 1)),
                &MaybeRelocatable::from(bigint!(3)),
            )
            .expect("Unexpected memory insert fail");
        vm.memory
            .insert(
                &MaybeRelocatable::from((2, 2)),
                &MaybeRelocatable::from(bigint!(5)),
            )
            .expect("Unexpected memory insert fail");
        vm.memory
            .insert(
                &MaybeRelocatable::from((2, 3)),
                &MaybeRelocatable::from(bigint!(7)),
            )
            .expect("Unexpected memory insert fail");
        vm.memory
            .insert(
                &MaybeRelocatable::from((3, 0)),
                elm_a.unwrap_or(&elm_a_default),
            )
            .expect("Unexpected memory insert fail");
        vm.memory
            .insert(
                &MaybeRelocatable::from((3, 1)),
                elm_b.unwrap_or(&elm_b_default),
            )
            .expect("Unexpected memory insert fail");

        vm.references = HashMap::from([
            (
                0,
                HintReference {
                    dereference: true,
                    register: Register::FP,
                    offset1: -5,
                    offset2: 0,
                    inner_dereference: false,
                    ap_tracking_data: None,
                    immediate: None,
                },
            ),
            (1, HintReference::new_simple(-4)),
            (2, HintReference::new_simple(-3)),
            (
                3,
                HintReference {
                    dereference: true,
                    register: Register::FP,
                    offset1: -2,
                    offset2: 0,
                    inner_dereference: false,
                    ap_tracking_data: None,
                    immediate: None,
                },
            ),
            (
                4,
                HintReference {
                    dereference: true,
                    register: Register::FP,
                    offset1: -1,
                    offset2: 0,
                    inner_dereference: false,
                    ap_tracking_data: None,
                    immediate: None,
                },
            ),
            (5, HintReference::new_simple(0)),
        ]);

        let mut ids = HashMap::<String, usize>::new();
        for (i, s) in [
            "is_elm_in_set",
            "index",
            "set_ptr",
            "elm_size",
            "elm_ptr",
            "set_end_ptr",
        ]
        .iter()
        .enumerate()
        {
            ids.insert(s.to_string(), i);
        }

        (vm, ids)
    }

    #[test]
    fn set_add_new_elem() {
        let (mut vm, ids) = init_vm_ids(None, None, None, None);
        let vm_proxy = &mut get_vm_proxy(&mut vm);
        assert_eq!(
            HINT_EXECUTOR.execute_hint(
                vm_proxy,
                exec_scopes_proxy_ref!(),
                HINT_CODE,
                &ids,
                &ApTracking::new()
            ),
            Ok(())
        );

        assert_eq!(
            vm.memory.get(&MaybeRelocatable::from((1, 0))),
            Ok(Some(&MaybeRelocatable::Int(bigint!(0))))
        )
    }

    #[test]
    fn set_add_already_exists() {
        let (mut vm, ids) = init_vm_ids(
            None,
            None,
            Some(&MaybeRelocatable::from(bigint!(1))),
            Some(&MaybeRelocatable::from(bigint!(3))),
        );
        let vm_proxy = &mut get_vm_proxy(&mut vm);
        assert_eq!(
            HINT_EXECUTOR.execute_hint(
                vm_proxy,
                exec_scopes_proxy_ref!(),
                HINT_CODE,
                &ids,
                &ApTracking::new()
            ),
            Ok(())
        );

        assert_eq!(
            vm.memory.get(&MaybeRelocatable::from((1, 0))),
            Ok(Some(&MaybeRelocatable::Int(bigint!(1))))
        );

        assert_eq!(
            vm.memory.get(&MaybeRelocatable::from((1, 1))),
            Ok(Some(&MaybeRelocatable::Int(bigint!(0))))
        )
    }

    #[test]
    fn elm_size_not_int() {
        let (mut vm, ids) = init_vm_ids(None, Some(&MaybeRelocatable::from((7, 8))), None, None);
        let vm_proxy = &mut get_vm_proxy(&mut vm);
        assert_eq!(
            HINT_EXECUTOR.execute_hint(
                vm_proxy,
                exec_scopes_proxy_ref!(),
                HINT_CODE,
                &ids,
                &ApTracking::new()
            ),
            Err(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 3))
            ))
        );
    }

    #[test]
    fn elm_size_negative() {
        let int = bigint!(-2);
        let (mut vm, ids) =
            init_vm_ids(None, Some(&MaybeRelocatable::Int(int.clone())), None, None);
        let vm_proxy = &mut get_vm_proxy(&mut vm);
        assert_eq!(
            HINT_EXECUTOR.execute_hint(
                vm_proxy,
                exec_scopes_proxy_ref!(),
                HINT_CODE,
                &ids,
                &ApTracking::new()
            ),
            Err(VirtualMachineError::BigintToUsizeFail)
        );
    }

    #[test]
    fn elm_size_zero() {
        let int = bigint!(0_i32);
        let (mut vm, ids) =
            init_vm_ids(None, Some(&MaybeRelocatable::Int(int.clone())), None, None);
        let vm_proxy = &mut get_vm_proxy(&mut vm);
        assert_eq!(
            HINT_EXECUTOR.execute_hint(
                vm_proxy,
                exec_scopes_proxy_ref!(),
                HINT_CODE,
                &ids,
                &ApTracking::new()
            ),
            Err(VirtualMachineError::ValueNotPositive(int))
        );
    }
    #[test]
    fn set_ptr_gt_set_end_ptr() {
        let (mut vm, ids) = init_vm_ids(Some(&MaybeRelocatable::from((2, 3))), None, None, None);
        let vm_proxy = &mut get_vm_proxy(&mut vm);
        assert_eq!(
            HINT_EXECUTOR.execute_hint(
                vm_proxy,
                exec_scopes_proxy_ref!(),
                HINT_CODE,
                &ids,
                &ApTracking::new()
            ),
            Err(VirtualMachineError::InvalidSetRange(
                MaybeRelocatable::from((2, 3)),
                MaybeRelocatable::from((2, 2)),
            ))
        );
    }

    #[test]
    fn find_elm_failed_ids_get_addres() {
        let (mut vm, ids) = init_vm_ids(None, None, None, None);
        vm.references.insert(0, HintReference::new_simple(-7));
        let vm_proxy = &mut get_vm_proxy(&mut vm);
        assert_eq!(
            HINT_EXECUTOR.execute_hint(
                vm_proxy,
                exec_scopes_proxy_ref!(),
                HINT_CODE,
                &ids,
                &ApTracking::new()
            ),
            Err(VirtualMachineError::FailedToGetIds)
        );
    }
}
