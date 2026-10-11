import * as THREE from "three";
import { GLTFLoader, type GLTF } from "three/examples/jsm/loaders/GLTFLoader.js";
import { clone as cloneSkinned } from "three/examples/jsm/utils/SkeletonUtils.js";

const modelCache = new Map<string, GLTF>();

export async function loadModel(modelUrl: string): Promise<GLTF> {
  const hit = modelCache.get(modelUrl);
  if (hit) return hit;
  const loader = new GLTFLoader();
  const gltf = await new Promise<GLTF>((resolve, reject) => {
    loader.load(modelUrl, resolve, undefined, reject);
  });
  modelCache.set(modelUrl, gltf);
  return gltf;
}

/** Clone a skinned GLTF scene so skeleton binds stay intact. */
export function clonePlayer(scene: THREE.Object3D): THREE.Object3D {
  return cloneSkinned(scene);
}

export async function loadSkinTexture(url: string): Promise<THREE.Texture> {
  return new Promise((resolve, reject) => {
    new THREE.TextureLoader().load(
      url,
      (texture) => {
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.flipY = false;
        texture.magFilter = THREE.NearestFilter;
        texture.minFilter = THREE.NearestFilter;
        resolve(texture);
      },
      undefined,
      reject,
    );
  });
}

function tuneSkinMaterial(mat: THREE.MeshStandardMaterial, texture: THREE.Texture, overlay: boolean) {
  mat.map = texture;
  mat.metalness = 0;
  mat.roughness = 1;
  mat.color.set(0xffffff);
  mat.toneMapped = false;
  mat.flatShading = true;
  mat.side = THREE.FrontSide;
  // GLTF ships alphaMode BLEND; that makes limbs see-through. Cutout instead.
  mat.transparent = overlay;
  mat.opacity = 1;
  mat.alphaTest = 0.5;
  mat.depthWrite = true;
  mat.depthTest = true;
  mat.needsUpdate = true;
}

export function applySkinTexture(model: THREE.Object3D, texture: THREE.Texture): void {
  model.traverse((child) => {
    if (!(child as THREE.Mesh).isMesh) return;
    const mesh = child as THREE.Mesh;
    const overlay = /layer|hat/i.test(mesh.name);
    mesh.renderOrder = overlay ? 1 : 0;
    const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    for (const mat of materials) {
      if (!(mat instanceof THREE.MeshStandardMaterial) || mat.name === "cape") continue;
      tuneSkinMaterial(mat, texture, overlay);
    }
  });
}

export function applyCapeTexture(
  model: THREE.Object3D,
  texture: THREE.Texture | null,
  transparent: THREE.Texture,
): void {
  model.traverse((child) => {
    if (!(child as THREE.Mesh).isMesh) return;
    const mesh = child as THREE.Mesh;
    const materials = Array.isArray(mesh.material) ? mesh.material : [mesh.material];
    for (const mat of materials) {
      if (!(mat instanceof THREE.MeshStandardMaterial) || mat.name !== "cape") continue;
      mat.map = texture || transparent;
      mat.visible = !!texture;
      mat.transparent = true;
      mat.opacity = 1;
      mat.metalness = 0;
      mat.color.set(0xffffff);
      mat.toneMapped = false;
      mat.flatShading = true;
      mat.roughness = 1;
      mat.side = THREE.DoubleSide;
      mat.alphaTest = 0.5;
      mat.depthWrite = true;
      mat.needsUpdate = true;
    }
  });
}

export function createTransparentTexture(): THREE.Texture {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = 1;
  const ctx = canvas.getContext("2d")!;
  ctx.clearRect(0, 0, 1, 1);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.flipY = false;
  texture.magFilter = THREE.NearestFilter;
  texture.minFilter = THREE.NearestFilter;
  return texture;
}
