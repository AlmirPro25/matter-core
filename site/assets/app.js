async function loadRelease() {
  const response = await fetch("release.json", { cache: "no-store" });
  if (!response.ok) {
    return;
  }

  const release = await response.json();
  const artifacts = release.artifacts || [];
  const artifact =
    artifacts.find((item) => item.name === "matter-core-beta-setup.exe") ||
    artifacts.find((item) => item.name === "matter-core-windows-x64.zip") ||
    artifacts[0];
  if (!artifact) {
    return;
  }

  const sizeMb = artifact.size_bytes / 1024 / 1024;
  document.getElementById("release-channel").textContent = release.channel || "beta";
  document.getElementById("release-version").textContent = release.version || "beta";
  document.getElementById("artifact-name").textContent = artifact.name;
  document.getElementById("artifact-size").textContent = `${sizeMb.toFixed(2)} MB`;
  document.getElementById("artifact-sha").textContent = artifact.sha256;
  document.getElementById("artifact-date").textContent = release.generated_at;

  const signature = document.getElementById("artifact-signature");
  if (signature) {
    signature.textContent = artifact.signed ? "assinado" : "nao assinado";
  }
}

loadRelease().catch(() => {});
