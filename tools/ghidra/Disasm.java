// Writes the disassembly of the functions containing the given RVAs to <outDir>/<rva>.asm.
// Output stays in re/ (gitignored); never paste it into tracked source.
// Usage: headless.ps1 Disasm.java re/exports/asm <rva> [<rva> ...]
// @category sekiro-rs

import java.io.File;
import java.io.PrintWriter;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.address.Address;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;

public class Disasm extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        File out = new File(args[0]);
        out.mkdirs();
        long base = currentProgram.getImageBase().getOffset();
        for (int i = 1; i < args.length; i++) {
            long rva = Long.parseLong(args[i].replace("0x", ""), 16);
            Function f = getFunctionContaining(toAddr(base + rva));
            if (f == null) {
                continue;
            }
            try (PrintWriter w = new PrintWriter(new File(out, String.format("%x.asm", rva)), "UTF-8")) {
                for (Instruction ins : currentProgram.getListing().getInstructions(f.getBody(), true)) {
                    Address a = ins.getAddress();
                    w.printf("%x\t%s%n", a.getOffset() - base, ins);
                }
            }
        }
    }
}
