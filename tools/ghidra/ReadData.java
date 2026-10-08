// Prints raw values at RVAs (as u32 hex, f32, and the first 16 bytes) into <outFile>.
// Output stays in re/ (gitignored).
// Usage: headless.ps1 ReadData.java re/exports/data.txt <rva> [<rva> ...]
// @category sekiro-rs

import java.io.FileOutputStream;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;

public class ReadData extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        long base = currentProgram.getImageBase().getOffset();
        try (PrintWriter w = new PrintWriter(new FileOutputStream(args[0], true))) {
            for (int i = 1; i < args.length; i++) {
                long rva = Long.parseLong(args[i].replace("0x", ""), 16);
                Address a = toAddr(base + rva);
                StringBuilder hex = new StringBuilder();
                for (int k = 0; k < 16; k++) {
                    hex.append(String.format("%02x ", getByte(a.add(k)) & 0xff));
                }
                int v = getInt(a);
                w.printf("%x\tu32=%08x\ti32=%d\tf32=%g\tf64=%g\t%s%n", rva, v, v, Float.intBitsToFloat(v),
                        Double.longBitsToDouble(getLong(a)), hex);
            }
        }
    }
}
