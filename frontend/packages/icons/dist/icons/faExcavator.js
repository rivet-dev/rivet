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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faExcavator.js
var require_faExcavator = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faExcavator.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "excavator";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e656";
    var svgPathData = "M160 64l0 96 142.9 0-64-96L160 64zM384 190.5l0 17.5 0 16 0 48c0 26.5-21.5 48-48 48L48 320c-26.5 0-48-21.5-48-48l0-64c0-26.5 21.5-48 48-48l48 0L96 48c0-26.5 21.5-48 48-48l94.9 0c21.4 0 41.4 10.7 53.3 28.5l78.4 117.7c8.8 13.1 13.4 28.6 13.4 44.4zM80 416c-8.8 0-16 7.2-16 16s7.2 16 16 16l256 0c8.8 0 16-7.2 16-16s-7.2-16-16-16L80 416zM0 432c0-44.2 35.8-80 80-80l256 0c44.2 0 80 35.8 80 80s-35.8 80-80 80L80 512c-44.2 0-80-35.8-80-80zM640 320l0 64 0 48c0 44.2-35.8 80-80 80l-78.1 0c-18.7 0-33.9-15.2-33.9-33.9c0-9 3.6-17.6 9.9-24L576 336l0-208L416 288l0-128L562.7 13.3c8.5-8.5 20-13.3 32-13.3c25 0 45.3 20.3 45.3 45.3L640 320z";
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
    exports.faExcavator = exports.definition;
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

// src/_entries/faExcavator.js
var import_faExcavator = __toESM(require_faExcavator());
var export_faExcavator = import_faExcavator.definition;
export {
  export_faExcavator as faExcavator
};
