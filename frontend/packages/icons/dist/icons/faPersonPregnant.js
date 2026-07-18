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

// src/node_modules/@fortawesome/free-solid-svg-icons/faPersonPregnant.js
var require_faPersonPregnant = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faPersonPregnant.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-pregnant";
    var width = 384;
    var height = 512;
    var aliases = [];
    var unicode = "e31e";
    var svgPathData = "M192 80a56 56 0 1 0 0-112 56 56 0 1 0 0 112zm80 321.2c8.6-4.5 16.8-10 24.3-16.5l4-3.4c22.6-19.4 35.7-47.7 35.7-77.6 0-35.9-18.8-69.1-49.6-87.6l-30.4-18.2 0-1.8c0-46.5-37.7-84.1-84.1-84.1-28.1 0-54.4 14.1-70 37.5L21.4 270.2c-9.8 14.7-5.8 34.6 8.9 44.4s34.6 5.8 44.4-8.9l29-43.5-30.5 113.5c-2.6 9.6-.6 19.9 5.5 27.8S94 416 104 416l8 0 0 96c0 17.7 14.3 32 32 32s32-14.3 32-32l0-96 32 0 0 96c0 17.7 14.3 32 32 32s32-14.3 32-32l0-110.8z";
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
    exports.faPersonPregnant = exports.definition;
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

// src/_entries/faPersonPregnant.js
var import_faPersonPregnant = __toESM(require_faPersonPregnant());
var export_faPersonPregnant = import_faPersonPregnant.definition;
export {
  export_faPersonPregnant as faPersonPregnant
};
