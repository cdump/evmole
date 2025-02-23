import parseTree.Contract;
import parseTree.cfg.Cfg;
import parseTree.cfg.BasicBlock;
import parseTree.cfg.BasicBlockType;
import com.google.gson.Gson;
import com.google.gson.JsonObject;

import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.stream.Collectors;

public class HelloEtherSolve {
    private static final long PROCESS_TIMEOUT_SECONDS = 90;
    private final Gson gson = new Gson();

    private List<Long[]> processContract(String bytecode) {
        try {
            Cfg cfg =  new Contract("Sample", bytecode, true)
                .getRuntimeCfg();

            List<Long[]> ret = new ArrayList<Long[]>();
            for (BasicBlock block : cfg) {
                if (block.getType() == BasicBlockType.EXIT) {
                    continue;
                }
                long start = block.getOffset();
                for (BasicBlock successor : block.getSuccessors()) {
                    if (successor.getType() == BasicBlockType.EXIT) {
                        continue;
                    }
                    long off = successor.getOffset();
                    ret.add(new Long[]{start, off});
                }
            }
            return ret;
        } catch (Exception e) {
            e.printStackTrace();
            return List.of();
        }
    }

    private List<Long[]> executeWithTimeout(String bytecode) {
        ExecutorService executor = Executors.newSingleThreadExecutor();
        try {
            Future<List<Long[]>> future = executor.submit(() -> processContract(bytecode));
            return future.get(PROCESS_TIMEOUT_SECONDS, TimeUnit.SECONDS);
        } catch (TimeoutException e) {
            return List.of();
        } catch (Exception e) {
            e.printStackTrace();
            return List.of();
        } finally {
            executor.shutdownNow();
        }
    }

    private void processFile(Path file, Map<String, Object[]> results) throws Exception {
        if (!Files.isRegularFile(file)) return;

        String content = Files.readString(file);
        JsonObject json = gson.fromJson(content, JsonObject.class);
        String bytecode = json.get("code").getAsString().substring(2);

        long startTime = System.nanoTime();
        List<Long[]> processResults = executeWithTimeout(bytecode);
        long timeUs = TimeUnit.NANOSECONDS.toMicros(System.nanoTime() - startTime);

        results.put(file.getFileName().toString(), new Object[]{timeUs, processResults});
    }

    public void execute(String[] args) throws Exception {
        if (args.length < 3 || !"flow".equals(args[0])) {
            System.out.println("Usage: self flow INPUT_DIR OUTPUT_FILE");
            System.exit(1);
        }

        Map<String, Object[]> results = new HashMap<>();
        Path inputDir = Paths.get(args[1]);
        Path outputFile = Paths.get(args[2]);

        try (DirectoryStream<Path> stream = Files.newDirectoryStream(inputDir)) {
            for (Path file : stream) {
                processFile(file, results);
            }
        }

        Files.writeString(outputFile, gson.toJson(results));
    }

    public static void main(String[] args) {
        try {
            new HelloEtherSolve().execute(args);
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
