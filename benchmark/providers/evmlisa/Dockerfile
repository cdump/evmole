FROM docker.io/gradle:jdk23

WORKDIR /app
RUN apt-get update && apt-get install -y git

# TMP branch without pinning
RUN git clone https://github.com/lisa-analyzer/evm-lisa && cd evm-lisa && git checkout basic-blocks && gradle shadowJar

# COPY evm-lisa-all.jar /app/
RUN mv ./evm-lisa/build/libs/evm-lisa-all.jar ./

COPY ./build.gradle /app/
COPY ./src/main/java/Main.java /app/src/main/java/
RUN gradle jar
ENTRYPOINT ["java", "-jar", "/app/build/libs/app.jar"]
