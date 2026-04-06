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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAngle90.js
var require_faAngle90 = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAngle90.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "angle-90";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e08d";
    var svgPathData = "M64 64c0-17.7-14.3-32-32-32S0 46.3 0 64L0 448c0 17.7 14.3 32 32 32l384 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L64 416 64 64zM96 97.6l0 48.3c31 3.6 60.4 12.5 87.4 25.7l25.3-41.1C174.2 113 136.2 101.6 96 97.6zm115.2 89.7c32.6 21 60.5 48.9 81.5 81.5l40.9-25.2c-25-39-58.2-72.2-97.2-97.2l-25.2 40.9zM334.1 384l48.3 0c-4-40.2-15.4-78.2-32.9-112.6l-41.1 25.3c13.2 26.9 22 56.3 25.7 87.4z";
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
    exports.faAngle90 = exports.definition;
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

// src/_entries/faAngle90.js
var import_faAngle90 = __toESM(require_faAngle90());
var export_faAngle90 = import_faAngle90.definition;
export {
  export_faAngle90 as faAngle90
};
