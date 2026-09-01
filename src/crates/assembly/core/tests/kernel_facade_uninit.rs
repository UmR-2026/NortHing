// 此文件因 A 类「未初始化断言」单测独占进程而独立成文件
// 不要向本文件添加任何会触发 init_core() / init_*() / 全局单例初始化的测试
// 违反即回归

use northhing_core::kernel_facade::kernel_facade;

#[test]
fn test_result_methods_return_error_before_init() {
    let facade = kernel_facade();
    match facade.coordinator() {
        Ok(_) => panic!("coordinator() should be Err before init_core"),
        Err(northhing_kernel_api::error::KernelError::Internal(_)) => {}
        Err(other) => panic!("expected KernelError::Internal, got {:?}", other),
    }
}
