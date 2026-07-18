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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRingDiamond.js
var require_faRingDiamond = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRingDiamond.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "ring-diamond";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e5ab";
    var svgPathData = "M122.5 13.3C126.6 5.1 134.9 0 144 0l96 0c9.1 0 17.4 5.1 21.5 13.3l24 48c3.5 7.1 3.4 15.5-.5 22.4l-30.4 54.8C329.9 164.4 384 235.9 384 320c0 106-86 192-192 192S0 426 0 320c0-84.1 54.1-155.6 129.4-181.6L99 83.7c-3.8-6.9-4-15.3-.5-22.4l24-48zM192 192a128 128 0 1 0 0 256 128 128 0 1 0 0-256z";
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
    exports.faRingDiamond = exports.definition;
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

// src/_entries/faRingDiamond.js
var import_faRingDiamond = __toESM(require_faRingDiamond());
var export_faRingDiamond = import_faRingDiamond.definition;
export {
  export_faRingDiamond as faRingDiamond
};
