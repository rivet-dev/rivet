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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faContainerStorage.js
var require_faContainerStorage = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faContainerStorage.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "container-storage";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "f4b7";
    var svgPathData = "M32 32C14.3 32 0 46.3 0 64S14.3 96 32 96l0 320c-17.7 0-32 14.3-32 32s14.3 32 32 32l576 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l0-320c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 32zM176 152l0 208c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-208c0-13.3 10.7-24 24-24s24 10.7 24 24zm112 0l0 208c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-208c0-13.3 10.7-24 24-24s24 10.7 24 24zm112 0l0 208c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-208c0-13.3 10.7-24 24-24s24 10.7 24 24zm112 0l0 208c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-208c0-13.3 10.7-24 24-24s24 10.7 24 24z";
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
    exports.faContainerStorage = exports.definition;
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

// src/_entries/faContainerStorage.js
var import_faContainerStorage = __toESM(require_faContainerStorage());
var export_faContainerStorage = import_faContainerStorage.definition;
export {
  export_faContainerStorage as faContainerStorage
};
