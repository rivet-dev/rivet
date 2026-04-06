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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCameraRetro.js
var require_faCameraRetro = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCameraRetro.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "camera-retro";
    var width = 512;
    var height = 512;
    var aliases = [128247];
    var unicode = "f083";
    var svgPathData = "M0 416l0-208 136.2 0c13.5-20.2 32-36.8 53.7-48L0 160 0 125.7c0-35.3 28.7-64 64-64l.1 0C65.3 45.1 79.1 32 96 32l32 0c16.9 0 30.7 13.1 31.9 29.7l32.1 0 51.2-23.8c8.4-3.9 17.6-6 26.9-6L448 32c35.3 0 64 28.7 64 64l0 64-190 0c21.7 11.2 40.2 27.8 53.7 48l136.2 0 0 208c0 35.3-28.7 64-64 64L64 480c-35.3 0-64-28.7-64-64zM256 192a96.1 96.1 0 1 0 0 192.1 96.1 96.1 0 1 0 0-192.1z";
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
    exports.faCameraRetro = exports.definition;
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

// src/_entries/faCameraRetro.js
var import_faCameraRetro = __toESM(require_faCameraRetro());
var export_faCameraRetro = import_faCameraRetro.definition;
export {
  export_faCameraRetro as faCameraRetro
};
