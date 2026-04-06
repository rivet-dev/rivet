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

// src/node_modules/@fortawesome/free-solid-svg-icons/faLaptopFile.js
var require_faLaptopFile = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faLaptopFile.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "laptop-file";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e51d";
    var svgPathData = "M64 64C64 28.7 92.7 0 128 0L416 0c35.3 0 64 28.7 64 64l0 48-64 0 0-48-288 0 0 192 112 0 0 96-163.2 0C34.4 352 0 317.6 0 275.2 0 264.6 8.6 256 19.2 256L64 256 64 64zM529.9 257.9c9 9 14.1 21.2 14.1 33.9L544 464c0 26.5-21.5 48-48 48l-160 0c-26.5 0-48-21.5-48-48l0-256c0-26.5 21.5-48 48-48l76.1 0c12.7 0 24.9 5.1 33.9 14.1 20 20 47.9 47.9 83.9 83.9zM416 272c0 8.8 7.2 16 16 16l60.1 0-76.1-76.1 0 60.1z";
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
    exports.faLaptopFile = exports.definition;
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

// src/_entries/faLaptopFile.js
var import_faLaptopFile = __toESM(require_faLaptopFile());
var export_faLaptopFile = import_faLaptopFile.definition;
export {
  export_faLaptopFile as faLaptopFile
};
