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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHeadSideGoggles.js
var require_faHeadSideGoggles = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHeadSideGoggles.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "head-side-goggles";
    var width = 512;
    var height = 512;
    var aliases = ["head-vr"];
    var unicode = "f6ea";
    var svgPathData = "M45.9 128C75.3 53.1 148.3 0 233.7 0L256 0c39.3 0 75.8 11.8 106.2 32L304 32c-61.9 0-112 50.1-112 112c0 17.2 3.9 33.5 10.8 48L32.2 192l-.2 0c-17.7 0-32-14.3-32-32s14.3-32 32-32l13.9 0zM304 256l203.1 0c3.2 6.2 4.9 13.1 4.9 20.2c0 24.2-19.6 43.8-43.8 43.8L448 320l0 64c0 35.3-28.7 64-64 64l-64 0 0 32c0 17.7-14.3 32-32 32l-160 0c-17.7 0-32-14.3-32-32l0-104.2c0-17-7.1-33-17.5-46.4C55 299.1 38.3 261.6 33.5 224l192.2 0c20.2 19.8 47.9 32 78.4 32zm0-192l112 0 0 160-112 0c-44.2 0-80-35.8-80-80s35.8-80 80-80zM480 224l-32 0 0-160 32 0c17.7 0 32 14.3 32 32l0 96c0 17.7-14.3 32-32 32z";
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
    exports.faHeadSideGoggles = exports.definition;
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

// src/_entries/faHeadVr.js
var import_faHeadSideGoggles = __toESM(require_faHeadSideGoggles());
var export_faHeadVr = import_faHeadSideGoggles.definition;
export {
  export_faHeadVr as faHeadVr
};
