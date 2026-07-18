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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faGlobeStand.js
var require_faGlobeStand = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faGlobeStand.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "globe-stand";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f5f6";
    var svgPathData = "M15 367c-9.4 9.4-9.4 24.6 0 33.9s24.6 9.4 33.9 0l18.5-18.5c34.3 27.7 74.9 43.8 116.5 48.3l0 33.3-96 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l240 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-96 0 0-33.3c49.1-5.3 96.8-26.7 134.4-64.3c81.7-81.7 87.1-211 16.1-298.9L401 49c9.4-9.4 9.4-24.6 0-33.9s-24.6-9.4-33.9 0L332.5 49.6c-9.4 9.4-9.4 24.6 0 33.9c68.7 68.7 68.7 180.2 0 248.9s-180.2 68.7-248.9 0c-9.4-9.4-24.6-9.4-33.9 0L15 367zm193-15a144 144 0 1 0 0-288 144 144 0 1 0 0 288z";
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
    exports.faGlobeStand = exports.definition;
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

// src/_entries/faGlobeStand.js
var import_faGlobeStand = __toESM(require_faGlobeStand());
var export_faGlobeStand = import_faGlobeStand.definition;
export {
  export_faGlobeStand as faGlobeStand
};
