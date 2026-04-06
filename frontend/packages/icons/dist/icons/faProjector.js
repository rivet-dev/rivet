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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faProjector.js
var require_faProjector = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faProjector.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "projector";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f8d6";
    var svgPathData = "M440 24c0-13.3-10.7-24-24-24s-24 10.7-24 24l0 80c0 13.3 10.7 24 24 24s24-10.7 24-24l0-80zM281 39c-9.4-9.4-24.6-9.4-33.9 0s-9.4 24.6 0 33.9l56 56c9.4 9.4 24.6 9.4 33.9 0s9.4-24.6 0-33.9L281 39zM585 73c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0L495 95c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0l56-56zm-9 119l-64 0c-26.7-20.1-60-32-96-32s-69.3 11.9-96 32L64 192c-35.3 0-64 28.7-64 64L0 384c0 35.3 28.7 64 64 64l9.9 39.8C77.5 502 90.3 512 105 512l14 0c14.7 0 27.5-10 31-24.2L160 448l160 0c26.7 20.1 60 32 96 32c24.4 0 47.5-5.5 68.2-15.2l5.7 23c3.6 14.2 16.4 24.2 31 24.2l14 0c14.7 0 27.5-10 31-24.2L576 448c35.3 0 64-28.7 64-64l0-128c0-35.3-28.7-64-64-64zM416 224a96 96 0 1 1 0 192 96 96 0 1 1 0-192zM64 320a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm128-32a32 32 0 1 1 0 64 32 32 0 1 1 0-64z";
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
    exports.faProjector = exports.definition;
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

// src/_entries/faProjector.js
var import_faProjector = __toESM(require_faProjector());
var export_faProjector = import_faProjector.definition;
export {
  export_faProjector as faProjector
};
