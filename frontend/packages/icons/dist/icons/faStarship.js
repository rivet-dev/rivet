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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStarship.js
var require_faStarship = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStarship.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "starship";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e039";
    var svgPathData = "M448 192a64 64 0 1 1 0 128 64 64 0 1 1 0-128zm0 256a192 192 0 1 0 0-384 192 192 0 1 0 0 384zm32-192a32 32 0 1 0 -64 0 32 32 0 1 0 64 0zM188.9 205.4L137 160l71 0c17.7 0 32-14.3 32-32s-14.3-32-32-32L32 96C14.3 96 0 110.3 0 128s14.3 32 32 32l32.1 0 65.5 57.3-2.2 .4C109.1 221.4 96 237.4 96 256s13.1 34.6 31.4 38.3l2.2 .4L64.1 352 32 352c-17.7 0-32 14.3-32 32s14.3 32 32 32l176 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-71 0 51.9-45.4 43 8.6c-5.2-18.9-7.9-38.7-7.9-59.2s2.7-40.3 7.9-59.2l-43 8.6z";
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
    exports.faStarship = exports.definition;
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

// src/_entries/faStarship.js
var import_faStarship = __toESM(require_faStarship());
var export_faStarship = import_faStarship.definition;
export {
  export_faStarship as faStarship
};
