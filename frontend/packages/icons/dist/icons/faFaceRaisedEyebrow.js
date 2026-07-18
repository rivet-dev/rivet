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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceRaisedEyebrow.js
var require_faFaceRaisedEyebrow = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceRaisedEyebrow.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-raised-eyebrow";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e388";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zm208.4-16a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm128 32a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM144 352c0 8.8 7.2 16 16 16l192 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-192 0c-8.8 0-16 7.2-16 16zM328.8 141.8c10.7-10.7 28.5-8.9 36.9 3.7l4.9 7.4c4.9 7.4 14.8 9.3 22.2 4.4s9.3-14.8 4.4-22.2l-4.9-7.4c-19.6-29.4-61.2-33.5-86.2-8.5l-29.5 29.5c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l29.5-29.5zM128 144c-8.8 0-16 7.2-16 16s7.2 16 16 16l64 0c8.8 0 16-7.2 16-16s-7.2-16-16-16l-64 0z";
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
    exports.faFaceRaisedEyebrow = exports.definition;
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

// src/_entries/faFaceRaisedEyebrow.js
var import_faFaceRaisedEyebrow = __toESM(require_faFaceRaisedEyebrow());
var export_faFaceRaisedEyebrow = import_faFaceRaisedEyebrow.definition;
export {
  export_faFaceRaisedEyebrow as faFaceRaisedEyebrow
};
