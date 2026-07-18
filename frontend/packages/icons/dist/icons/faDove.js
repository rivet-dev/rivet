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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDove.js
var require_faDove = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDove.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dove";
    var width = 512;
    var height = 512;
    var aliases = [128330];
    var unicode = "f4ba";
    var svgPathData = "M496 64c13.2 0 20.7 15.1 12.8 25.6L480 128 480 304c0 79.5-64.5 144-144 144l-112 0-46.3 46.3c-10.4 10.4-26.5 12.4-39.1 4.8L41.5 440.9c-17-10.2-15-35.5 3.4-42.9L160 352C23.8 311.1 7.5 169.8 22 95.7 25.6 77.9 45.3 71.4 61.3 80.2L320 224 320 144c0-44.2 35.8-80 80-80l96 0zm-96 56a24 24 0 1 0 0 48 24 24 0 1 0 0-48zM182.5-9.6c12.4-13.7 33.3-8.9 42.5 7.1l56.4 98.3c-5.8 14.4-9.2 30.1-9.4 46.5L138.1 68c10.1-31.6 27-58.4 44.4-77.6z";
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
    exports.faDove = exports.definition;
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

// src/_entries/faDove.js
var import_faDove = __toESM(require_faDove());
var export_faDove = import_faDove.definition;
export {
  export_faDove as faDove
};
