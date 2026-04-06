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

// src/node_modules/@fortawesome/free-solid-svg-icons/faWineBottle.js
var require_faWineBottle = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faWineBottle.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "wine-bottle";
    var width = 320;
    var height = 512;
    var aliases = [];
    var unicode = "f72f";
    var svgPathData = "M80-32c0-17.7 14.3-32 32-32l96 0c17.7 0 32 14.3 32 32 0 11.8-6.4 22.2-16 27.7l0 145.6c56.5 24.7 96 81.1 96 146.7l0 224c0 35.3-28.7 64-64 64L64 576c-35.3 0-64-28.7-64-64L0 288c0-65.6 39.5-122 96-146.7L96-4.3C86.4-9.8 80-20.2 80-32zM96 288c-17.7 0-32 14.3-32 32l0 128c0 17.7 14.3 32 32 32l128 0c17.7 0 32-14.3 32-32l0-128c0-17.7-14.3-32-32-32L96 288z";
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
    exports.faWineBottle = exports.definition;
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

// src/_entries/faWineBottle.js
var import_faWineBottle = __toESM(require_faWineBottle());
var export_faWineBottle = import_faWineBottle.definition;
export {
  export_faWineBottle as faWineBottle
};
