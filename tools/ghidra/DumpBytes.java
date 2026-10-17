// args: <outfile> <seg:off start> <length>   -> writes raw bytes
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import java.io.*;
public class DumpBytes extends GhidraScript {
  @Override public void run() throws Exception {
    String[] a = getScriptArgs();
    Address s = currentProgram.getAddressFactory().getAddress(a[1]);
    int n = Integer.parseInt(a[2]);
    byte[] b = new byte[n];
    currentProgram.getMemory().getBytes(s, b);
    try (FileOutputStream f = new FileOutputStream(a[0])) { f.write(b); }
    println("dumped " + n + " bytes from " + s);
  }
}
