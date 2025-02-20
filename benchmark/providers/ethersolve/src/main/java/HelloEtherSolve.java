import parseTree.Contract;
import com.google.gson.Gson;
import com.google.gson.JsonObject;

import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.stream.Collectors;

public class HelloEtherSolve {
    private static final long PROCESS_TIMEOUT_SECONDS = 10;
    private final Gson gson = new Gson();

    private List<Long[]> processContract(String bytecode) {
        try {
            return new Contract("Sample", bytecode, true)
                .getRuntimeCfg()
                .getSuccessorsMap()
                .entrySet()
                .stream()
                .flatMap(entry -> entry.getValue().stream()
                        .map(successor -> new Long[]{entry.getKey(), successor}))
                .collect(Collectors.toList());
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
        long timeMs = TimeUnit.NANOSECONDS.toMillis(System.nanoTime() - startTime);

        results.put(file.getFileName().toString(), new Object[]{timeMs, processResults});
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
