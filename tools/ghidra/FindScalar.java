// Lists instructions whose operands contain any of the given scalar values, with the containing
// function RVA, into <outFile>. Output stays in re/ (gitignored).
// Usage: headless.ps1 FindScalar.java re/exports/scalars.txt <hexvalue> [<hexvalue> ...]
// @category sekiro-rs

import java.io.PrintWriter;
import java.util.HashSet;
import java.util.Set;

import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.Instruction;
import ghidra.program.model.listing.InstructionIterator;
import ghidra.program.model.scalar.Scalar;

public class FindScalar extends GhidraScript {

    @Override
    protected void run() throws Exception {
        String[] args = getScriptArgs();
        Set<Long> wanted = new HashSet<>();
        for (int i = 1; i < args.length; i++) {
            wanted.add(Long.parseLong(args[i].replace("0x", ""), 16));
        }
        long base = currentProgram.getImageBase().getOffset();
        try (PrintWriter w = new PrintWriter(args[0], "UTF-8")) {
            InstructionIterator it = currentProgram.getListing().getInstructions(true);
            while (it.hasNext() && !monitor.isCancelled()) {
                Instruction ins = it.next();
                for (int op = 0; op < ins.getNumOperands(); op++) {
                    for (Object o : ins.getOpObjects(op)) {
                        if (o instanceof Scalar && wanted.contains(((Scalar) o).getUnsignedValue())) {
                            Function f = getFunctionContaining(ins.getAddress());
                            long frva = f == null ? -1 : f.getEntryPoint().getOffset() - base;
                            w.printf("%x\t%x\t%s%n", ins.getAddress().getOffset() - base, frva, ins);
                        }
                    }
                }
            }
        }
    }
}
