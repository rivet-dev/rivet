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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBlockBrick.js
var require_faBlockBrick = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBlockBrick.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "block-brick";
    var width = 448;
    var height = 512;
    var aliases = ["wall-brick"];
    var unicode = "e3db";
    var svgPathData = "M96 32l0 80 256 0 0-80L96 32zM64 112l0-80C28.7 32 0 60.7 0 96l0 16 64 0zM0 144l0 96 208 0 0-96L0 144zM0 368l64 0 0-96L0 272l0 96zm0 32l0 16c0 35.3 28.7 64 64 64l144 0 0-80L0 400zm240 0l0 80 144 0c35.3 0 64-28.7 64-64l0-16-208 0zm208-32l0-96-64 0 0 96 64 0zm-96 0l0-96L96 272l0 96 256 0zm96-224l-208 0 0 96 208 0 0-96zm0-32l0-16c0-35.3-28.7-64-64-64l0 80 64 0z";
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
    exports.faBlockBrick = exports.definition;
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

// src/_entries/faWallBrick.js
var import_faBlockBrick = __toESM(require_faBlockBrick());
var export_faWallBrick = import_faBlockBrick.definition;
export {
  export_faWallBrick as faWallBrick
};
