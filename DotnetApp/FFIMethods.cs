using System.Runtime.InteropServices;

public static class FFIMethods
{
    [DllImport("rust_lib.dll", EntryPoint = "add", CallingConvention = CallingConvention.Cdecl)]
    public static unsafe extern ulong Add(ulong left, ulong right);

    [DllImport("rust_lib.dll", EntryPoint = "create_ffi_type", CallingConvention = CallingConvention.Cdecl)]
    public static unsafe extern FFIType CreateFFIType(ulong x, ulong y);

    public struct FFIType
    {
        public int X;
        public int Y;
    }
}
