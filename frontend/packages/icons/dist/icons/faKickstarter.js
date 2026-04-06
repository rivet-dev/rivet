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

// src/node_modules/@fortawesome/free-brands-svg-icons/faKickstarter.js
var require_faKickstarter = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faKickstarter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "kickstarter";
    var width = 448;
    var height = 512;
    var aliases = ["square-kickstarter"];
    var unicode = "f3bb";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l320 0c35.3 0 64-28.7 64-64l0-320c0-35.3-28.7-64-64-64L64 32zM320.8 233l-23.3 23.1 23.3 22.9c24.1 23.9 24.1 63 0 86.9s-63.4 23.9-87.6 0l-8.5-8.4c-11.3 16-29.7 26.5-50.9 26.5-34.1 0-61.9-27.5-61.9-61.4l0-133.2c0-33.8 27.7-61.4 61.9-61.4 21.1 0 39.6 10.5 50.9 26.5l8.5-8.4c24.1-23.9 63.4-23.9 87.6 0s24.1 63 0 86.9z";
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
    exports.faKickstarter = exports.definition;
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

// src/_entries/faKickstarter.js
var import_faKickstarter = __toESM(require_faKickstarter());
var export_faKickstarter = import_faKickstarter.definition;
export {
  export_faKickstarter as faKickstarter
};
