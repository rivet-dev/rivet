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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faCitrus.js
var require_faCitrus = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faCitrus.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "citrus";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e2f4";
    var svgPathData = "M304 128l80 0c53 0 96-43 96-96l0-16c0-8.8-7.2-16-16-16L384 0c-53 0-96 43-96 96l0 16c0 8.8 7.2 16 16 16zM256 96c0-23.1 6.1-44.7 16.8-63.4c-5.5-.4-11.1-.6-16.8-.6C123.5 32 16 139.5 16 272s107.5 240 240 240s240-107.5 240-240c0-49.5-15-95.5-40.7-133.7C435 152 410.4 160 384 160l-80 0c-26.5 0-48-21.5-48-48l0-16z";
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
    exports.faCitrus = exports.definition;
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

// src/_entries/faCitrus.js
var import_faCitrus = __toESM(require_faCitrus());
var export_faCitrus = import_faCitrus.definition;
export {
  export_faCitrus as faCitrus
};
