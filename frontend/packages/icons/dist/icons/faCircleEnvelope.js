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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleEnvelope.js
var require_faCircleEnvelope = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCircleEnvelope.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "circle-envelope";
    var width = 512;
    var height = 512;
    var aliases = ["envelope-circle"];
    var unicode = "e10c";
    var svgPathData = "M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM128 192c0-17.7 14.3-32 32-32l192 0c17.7 0 32 14.3 32 32l0 9.3L264.7 255.6c-2.7 1.2-5.7 1.9-8.7 1.9s-5.9-.6-8.7-1.9L128 201.3l0-9.3zm149.9 92.7L384 236.5l0 83.5c0 17.7-14.3 32-32 32l-192 0c-17.7 0-32-14.3-32-32l0-83.5 106.1 48.3c6.9 3.1 14.3 4.8 21.9 4.8s15-1.6 21.9-4.8z";
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
    exports.faCircleEnvelope = exports.definition;
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

// src/_entries/faCircleEnvelope.js
var import_faCircleEnvelope = __toESM(require_faCircleEnvelope());
var export_faCircleEnvelope = import_faCircleEnvelope.definition;
export {
  export_faCircleEnvelope as faCircleEnvelope
};
