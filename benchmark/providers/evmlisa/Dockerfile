FROM docker.io/gradle:jdk23

WORKDIR /app
RUN apt-get update && apt-get install -y git

# 24 feb 2025 commit
RUN git clone https://github.com/lisa-analyzer/evm-lisa && cd evm-lisa && git checkout f12cc46d6a87de6c5d553273d841c6d35564b4cd && gradle shadowJar

# COPY evm-lisa-all.jar /app/
RUN mv ./evm-lisa/build/libs/evm-lisa-all.jar ./

COPY ./build.gradle /app/
COPY ./src/main/java/Main.java /app/src/main/java/
RUN gradle jar
ENTRYPOINT ["java", "-jar", "/app/build/libs/app.jar"]
