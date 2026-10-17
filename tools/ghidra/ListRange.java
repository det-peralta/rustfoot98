// args: <outfile> <startAddr> <endAddr>  -> disassembly listing
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import java.io.*;
public class ListRange extends GhidraScript {
  @Override public void run() throws Exception {
    String[] a = getScriptArgs();
    Address s = currentProgram.getAddressFactory().getAddress(a[1]), e = currentProgram.getAddressFactory().getAddress(a[2]);
    try (PrintWriter w = new PrintWriter(new FileWriter(a[0], true))) {
      for (Instruction i = getInstructionAt(s) != null ? getInstructionAt(s) : getInstructionAfter(s); i != null && i.getAddress().compareTo(e) <= 0; i = i.getNext()) {
        StringBuilder b = new StringBuilder();
        for (byte x : i.getBytes()) b.append(String.format("%02x", x));
        w.printf("%s  %-14s %s%n", i.getAddress(), b, i);
      }
    }
  }
}
