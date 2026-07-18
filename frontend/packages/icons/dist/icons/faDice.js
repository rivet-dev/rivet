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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDice.js
var require_faDice = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDice.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dice";
    var width = 512;
    var height = 512;
    var aliases = [127922];
    var unicode = "f522";
    var svgPathData = "M141.4 2.3C103-8 63.5 14.8 53.3 53.2L2.5 242.7C-7.8 281.1 15 320.6 53.4 330.9l189.5 50.8c38.4 10.3 77.9-12.5 88.2-50.9l50.8-189.5c10.3-38.4-12.5-77.9-50.9-88.2L141.4 2.3zm23 205.7a32 32 0 1 1 55.4-32 32 32 0 1 1 -55.4 32zM79.2 220.3a32 32 0 1 1 32 55.4 32 32 0 1 1 -32-55.4zm185 96.4a32 32 0 1 1 -32-55.4 32 32 0 1 1 32 55.4zm9-208.4a32 32 0 1 1 32 55.4 32 32 0 1 1 -32-55.4zm-121 14.4a32 32 0 1 1 -32-55.4 32 32 0 1 1 32 55.4zM418 192L377.4 343.2c-17.2 64-83 102-147 84.9l-38.3-10.3 0 30.2c0 35.3 28.7 64 64 64l192 0c35.3 0 64-28.7 64-64l0-192c0-35.3-28.7-64-64-64L418 192z";
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
    exports.faDice = exports.definition;
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

// src/_entries/faDice.js
var import_faDice = __toESM(require_faDice());
var export_faDice = import_faDice.definition;
export {
  export_faDice as faDice
};
