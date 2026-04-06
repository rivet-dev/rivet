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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGrapes.js
var require_faGrapes = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGrapes.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "grapes";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e306";
    var svgPathData = "M346.7 6C337.6 17 320 42.3 320 72c0 40 15.3 55.3 40 80s40 40 80 40c29.7 0 55-17.6 66-26.7c4-3.3 6-8.2 6-13.3s-2-10-6-13.2c-11.4-9.1-38.3-26.8-74-26.8c-32 0-40 8-40 8s8-8 8-40c0-35.7-17.7-62.6-26.8-74C370 2 365.1 0 360 0s-10 2-13.3 6zM304 152a56 56 0 1 0 -112 0 56 56 0 1 0 112 0zM112 232a56 56 0 1 0 0-112 56 56 0 1 0 0 112zm32 80A56 56 0 1 0 32 312a56 56 0 1 0 112 0zM112 456A56 56 0 1 0 0 456a56 56 0 1 0 112 0zm144-32a56 56 0 1 0 -112 0 56 56 0 1 0 112 0zm80 32a56 56 0 1 0 0-112 56 56 0 1 0 0 112zM280 288a56 56 0 1 0 -112 0 56 56 0 1 0 112 0zm80 32a56 56 0 1 0 0-112 56 56 0 1 0 0 112z";
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
    exports.faGrapes = exports.definition;
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

// src/_entries/faGrapes.js
var import_faGrapes = __toESM(require_faGrapes());
var export_faGrapes = import_faGrapes.definition;
export {
  export_faGrapes as faGrapes
};
