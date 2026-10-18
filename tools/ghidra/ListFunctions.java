// args: <listfile: "seg:off size" per line> <outfile> -> concatenated disassembly of every listed function
import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.*;
import ghidra.program.model.listing.*;
import java.io.*;
import java.nio.file.*;
public class ListFunctions extends GhidraScript {
  @Override public void run() throws Exception {
    String[] a = getScriptArgs();
    AddressFactory af = currentProgram.getAddressFactory();
    try (PrintWriter w = new PrintWriter(new FileWriter(a[1]))) {
      for (String l : Files.readAllLines(Paths.get(a[0]))) {
        String[] p = l.trim().split("\\s+"); if (p.length < 2) continue;
        Address s = af.getAddress(p[0]); long size = Long.parseLong(p[1]);
        w.println("### " + p[0]);
        Address e = s.add(size - 1);
        for (Instruction i = getInstructionAt(s) != null ? getInstructionAt(s) : getInstructionAfter(s); i != null && i.getAddress().compareTo(e) <= 0; i = i.getNext()) {
          StringBuilder b = new StringBuilder();
          for (byte x : i.getBytes()) b.append(String.format("%02x", x));
          w.printf("%s  %-14s %s%n", i.getAddress(), b, i);
        }
      }
    }
  }
}
