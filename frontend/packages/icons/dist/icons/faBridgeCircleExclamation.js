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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBridgeCircleExclamation.js
var require_faBridgeCircleExclamation = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBridgeCircleExclamation.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bridge-circle-exclamation";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e4ca";
    var svgPathData = "M32 32C14.3 32 0 46.3 0 64S14.3 96 32 96l8 0 0 64-40 0 0 112c37.6 9.4 64 43.2 64 82l0 94c0 17.7 14.3 32 32 32l32 0c17.7 0 32-14.3 32-32l0-64c0-53 43-96 96-96 6.4 0 12.7 .6 18.7 1.8 34.7-49.5 92.2-81.8 157.3-81.8 28.6 0 55.6 6.2 80 17.4l0-65.4-40 0 0-64 8 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 32zM424 96l0 64-80 0 0-64 80 0zM296 96l0 64-80 0 0-64 80 0zM88 96l80 0 0 64-80 0 0-64zM432 544a144 144 0 1 0 0-288 144 144 0 1 0 0 288zm0-100a20 20 0 1 1 0 40 20 20 0 1 1 0-40zm0-140c8.8 0 16 7.2 16 16l0 80c0 8.8-7.2 16-16 16s-16-7.2-16-16l0-80c0-8.8 7.2-16 16-16z";
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
    exports.faBridgeCircleExclamation = exports.definition;
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

// src/_entries/faBridgeCircleExclamation.js
var import_faBridgeCircleExclamation = __toESM(require_faBridgeCircleExclamation());
var export_faBridgeCircleExclamation = import_faBridgeCircleExclamation.definition;
export {
  export_faBridgeCircleExclamation as faBridgeCircleExclamation
};
