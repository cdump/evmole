import com.google.gson.Gson;
import com.google.gson.JsonObject;

import java.io.*;

import java.nio.file.*;
import java.util.*;
import java.util.concurrent.*;
import java.util.stream.Collectors;
import java.util.List;

import it.unipr.EVMLiSA;

class ProcessTimeoutExecutor {
    private static final long TIMEOUT_SECONDS = 90;

    public List<Long[]> executeWithTimeout(String input) throws Exception {
        Process process = null;
        ExecutorService executor = null;

        try {
            ProcessBuilder pb = new ProcessBuilder(
                "java",
                "-cp", System.getProperty("java.class.path"),
                getClass().getName(),
                "subprocess"
            );

            process = pb.start();
            Process finalProcess = process;

            executor = Executors.newFixedThreadPool(2);

            Future<?> inputFuture = executor.submit(() -> {
                try (OutputStreamWriter writer = new OutputStreamWriter(finalProcess.getOutputStream())) {
                    writer.write(input);
                    writer.flush();
                } catch (IOException e) {
                    throw new RuntimeException("Failed to write input to process", e);
                }
            });

            Future<List<Long[]>> outputFuture = executor.submit(() -> {
                try (ObjectInputStream ois = new ObjectInputStream(finalProcess.getErrorStream())) {
                    @SuppressWarnings("unchecked")
                    List<Long[]> result = (List<Long[]>) ois.readObject();
                    return result;
                } catch (Exception e) {
                    throw new RuntimeException("Failed to read process output", e);
                }
            });

            inputFuture.get(TIMEOUT_SECONDS, TimeUnit.SECONDS);
            return outputFuture.get(TIMEOUT_SECONDS, TimeUnit.SECONDS);

        } catch (TimeoutException e) {
            System.out.println("Process execution timed out");
            return null;
        } catch (Exception e) {
            System.out.println("Process execution failed: " + e.getMessage());
            return null;
        } finally {
            if (executor != null) {
                executor.shutdownNow();
            }
            if (process != null) {
                process.destroyForcibly();
                process.waitFor(1, TimeUnit.SECONDS);
            }
        }
    }

    public static void main(String[] args) {
        if (args.length > 0 && args[0].equals("subprocess")) {
            try {
                BufferedReader reader = new BufferedReader(new InputStreamReader(System.in));
                String input = reader.readLine();

                List<Long[]> result = processContract(input);

                ObjectOutputStream oos = new ObjectOutputStream(System.err);
                oos.writeObject(result);
                oos.flush();
            } catch (Exception e) {
                System.err.println("Subprocess error: " + e.getMessage());
                System.exit(1);
            }
            System.exit(0);
        }
    }

    private static List<Long[]> processContract(String bytecode) {
        try {
            return new EVMLiSA().computeBasicBlocks(bytecode);
        } catch (Exception e) {
            e.printStackTrace();
            return List.of();
        }
    }
}

public class Main {
    private static final long PROCESS_TIMEOUT_SECONDS = 90;
    private final Gson gson = new Gson();

    private List<Long[]> executeWithTimeout(String bytecode) {
        ProcessTimeoutExecutor executor = new ProcessTimeoutExecutor();
        try {
            List<Long[]> result = executor.executeWithTimeout(bytecode);
            if (result == null) {
                return List.of();
            } else {
                return result;
            }
        } catch (Exception e) {
            return List.of();
        }
    }

    private void processFile(Path file, Map<String, Object[]> results) throws Exception {
        if (!Files.isRegularFile(file)) return;

        String content = Files.readString(file);
        JsonObject json = gson.fromJson(content, JsonObject.class);
        String bytecode = json.get("code").getAsString(); // NEED 0x prefix for evmlisa

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
            int a = 0;
            for (Path file : stream) {
                a += 1;
                System.out.println(a);
                System.out.println(file);
                processFile(file, results);
            }
        }

        Files.writeString(outputFile, gson.toJson(results));
    }

    public static void main(String[] args) {
        try {
            new Main().execute(args);
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
