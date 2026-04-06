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

// src/node_modules/@fortawesome/free-solid-svg-icons/faTaurus.js
var require_faTaurus = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faTaurus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "taurus";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e84f";
    var svgPathData = "M336-16c17.7 0 32 14.3 32 32 0 58.5-28.6 110.3-72.5 142.3 53.2 34.1 88.5 93.8 88.5 161.7 0 106-86 192-192 192S0 426 0 320C0 252.1 35.3 192.4 88.5 158.3 44.6 126.3 16 74.5 16 16 16-1.7 30.3-16 48-16S80-1.7 80 16c0 61.9 50.1 112 112 112S304 77.9 304 16c0-17.7 14.3-32 32-32zM192 192a128 128 0 1 0 0 256 128 128 0 1 0 0-256z";
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
    exports.faTaurus = exports.definition;
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

// src/_entries/faTaurus.js
var import_faTaurus = __toESM(require_faTaurus());
var export_faTaurus = import_faTaurus.definition;
export {
  export_faTaurus as faTaurus
};
