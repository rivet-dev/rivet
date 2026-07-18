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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faScalpel.js
var require_faScalpel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faScalpel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "scalpel";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f61d";
    var svgPathData = "M387.4 25.1C400.8 9.2 420.6 0 441.4 0c39 0 70.6 31.6 70.6 70.6l0 4c0 17.6-6.2 34.6-17.6 48.1L369.3 271c-9.1 10.8-22.5 17-36.7 17L200 288c-6.2 0-11.9-3.6-14.5-9.3s-1.7-12.3 2.3-17.1L387.4 25.1zM169.4 320l6.6 0 96 0 16 0 0 16c0 74.9-65.5 118.6-126.1 142.9C100.1 503.6 34.1 512 16 512c-6.5 0-12.3-3.9-14.8-9.9s-1.1-12.9 3.5-17.4l160-160 4.7-4.7z";
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
    exports.faScalpel = exports.definition;
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

// src/_entries/faScalpel.js
var import_faScalpel = __toESM(require_faScalpel());
var export_faScalpel = import_faScalpel.definition;
export {
  export_faScalpel as faScalpel
};
