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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBackpack.js
var require_faBackpack = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBackpack.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "backpack";
    var width = 448;
    var height = 512;
    var aliases = [127890];
    var unicode = "f5d4";
    var svgPathData = "M192 48l64 0c4.4 0 8 3.6 8 8l0 40-80 0 0-40c0-4.4 3.6-8 8-8zm-56 8l0 40-8 0C57.3 96 0 153.3 0 224L0 448c0 35.3 28.7 64 64 64l0-112 0-48c0-35.3 28.7-64 64-64l192 0c35.3 0 64 28.7 64 64l0 48 0 112c35.3 0 64-28.7 64-64l0-224c0-70.7-57.3-128-128-128l-8 0 0-40c0-30.9-25.1-56-56-56L192 0c-30.9 0-56 25.1-56 56zM352 512l0-96L96 416l0 96 256 0zM144 176l160 0c8.8 0 16 7.2 16 16s-7.2 16-16 16l-160 0c-8.8 0-16-7.2-16-16s7.2-16 16-16zM352 384l0-32c0-17.7-14.3-32-32-32l-192 0c-17.7 0-32 14.3-32 32l0 32 256 0z";
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
    exports.faBackpack = exports.definition;
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

// src/_entries/faBackpack.js
var import_faBackpack = __toESM(require_faBackpack());
var export_faBackpack = import_faBackpack.definition;
export {
  export_faBackpack as faBackpack
};
