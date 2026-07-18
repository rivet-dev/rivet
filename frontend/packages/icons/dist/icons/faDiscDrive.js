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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDiscDrive.js
var require_faDiscDrive = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDiscDrive.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "disc-drive";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f8b5";
    var svgPathData = "M32 96c0-35.3 28.7-64 64-64l320 0c35.3 0 64 28.7 64 64l0 320c17.7 0 32 14.3 32 32s-14.3 32-32 32l-64 0L96 480l-64 0c-17.7 0-32-14.3-32-32s14.3-32 32-32L32 96zM224 256a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm192 0A160 160 0 1 0 96 256a160 160 0 1 0 320 0z";
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
    exports.faDiscDrive = exports.definition;
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

// src/_entries/faDiscDrive.js
var import_faDiscDrive = __toESM(require_faDiscDrive());
var export_faDiscDrive = import_faDiscDrive.definition;
export {
  export_faDiscDrive as faDiscDrive
};
