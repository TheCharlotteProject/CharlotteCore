/// Performs ISA specific initialization.
/// Should normally be implemented by a ZST.
trait Init {
    fn init();
}