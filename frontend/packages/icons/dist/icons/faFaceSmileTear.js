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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSmileTear.js
var require_faFaceSmileTear = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFaceSmileTear.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "face-smile-tear";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e393";
    var svgPathData = "M0 256a256 256 0 1 1 512 0A256 256 0 1 1 0 256zm141.5 67.9c-6.7 5.8-7.4 15.9-1.6 22.6c22.2 25.7 61 53.5 116.1 53.5c8.8 0 16-7.2 16-16s-7.2-16-16-16c-43.4 0-74-21.8-91.9-42.5c-5.8-6.7-15.9-7.4-22.6-1.6zM352 416c26.5 0 48-21 48-47c0-20-28.6-60.4-41.6-77.7c-3.2-4.4-9.6-4.4-12.8 0C332.4 308.6 304 349 304 369c0 26 21.5 47 48 47zM208.4 208a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zm128 32a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faFaceSmileTear = exports.definition;
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

// src/_entries/faFaceSmileTear.js
var import_faFaceSmileTear = __toESM(require_faFaceSmileTear());
var export_faFaceSmileTear = import_faFaceSmileTear.definition;
export {
  export_faFaceSmileTear as faFaceSmileTear
};
