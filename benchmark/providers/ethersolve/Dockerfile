FROM docker.io/gradle:jdk23

WORKDIR /app
COPY ./build.gradle /app/
COPY ./src/main/java/HelloEtherSolve.java /app/src/main/java/
RUN gradle build
ENTRYPOINT ["java", "-jar", "/app/build/libs/app.jar"]
