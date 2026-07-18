var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faImagePolaroid.js
var require_faImagePolaroid = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faImagePolaroid.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "image-polaroid";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f8c4";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM96 96l256 0c17.7 0 32 14.3 32 32l0 192c0 17.7-14.3 32-32 32L96 352c-17.7 0-32-14.3-32-32l0-192c0-17.7 14.3-32 32-32zm152 64c-5.3 0-10.2 2.6-13.2 6.9l-52.4 76.3-9.6-12.8c-3-4-7.8-6.4-12.8-6.4s-9.8 2.4-12.8 6.4l-48 64c-3.6 4.8-4.2 11.3-1.5 16.8s8.3 8.8 14.3 8.8l48 0 48 0 40 0 88 0c5.9 0 11.4-3.3 14.2-8.6s2.4-11.6-1-16.5l-88-128c-3-4.3-7.9-6.9-13.2-6.9zM128 192a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faImagePolaroid = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faImagePolaroid.js
var import_faImagePolaroid = __toESM(require_faImagePolaroid());
var export_faImagePolaroid = import_faImagePolaroid.definition;
export {
  export_faImagePolaroid as faImagePolaroid
};
