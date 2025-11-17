using System.Runtime.InteropServices;

public static class FFIMethods
{
    [StructLayout(LayoutKind.Sequential)]
    public struct FFITy
    {
        public ulong X;
        public ulong Y;
    }

    [DllImport("rust_lib.dll", EntryPoint = "add", CallingConvention = CallingConvention.Cdecl)]
    public static extern FFITy Add(ulong x, ulong y);

}
