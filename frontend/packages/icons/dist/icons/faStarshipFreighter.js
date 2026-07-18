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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStarshipFreighter.js
var require_faStarshipFreighter = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStarshipFreighter.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "starship-freighter";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e03a";
    var svgPathData = "M224 480C100.3 480 0 379.7 0 256S100.3 32 224 32c42.3 0 84.2 7.9 123.6 23.2l207.9 80.9c12.3 4.8 20.4 16.6 20.4 29.8l0 42.1c0 8.8-7.2 16-16 16l-114.3 0L432 224c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l13.7 0L560 288c8.8 0 16 7.2 16 16l0 42.1c0 13.2-8.1 25-20.4 29.8l-82.1 31.9c4.1 7.1 6.5 15.3 6.5 24.1c0 26.5-21.5 48-48 48l-80 0c-11.5 0-21.6-6.1-27.3-15.2C292.2 474.9 258.2 480 224 480zm0-272a48 48 0 1 1 0 96 48 48 0 1 1 0-96zm0 128a80 80 0 1 0 0-160 80 80 0 1 0 0 160zM80 272a16 16 0 1 0 0-32 16 16 0 1 0 0 32zm48-112a16 16 0 1 0 -32 0 16 16 0 1 0 32 0zM112 368a16 16 0 1 0 0-32 16 16 0 1 0 0 32z";
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
    exports.faStarshipFreighter = exports.definition;
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

// src/_entries/faStarshipFreighter.js
var import_faStarshipFreighter = __toESM(require_faStarshipFreighter());
var export_faStarshipFreighter = import_faStarshipFreighter.definition;
export {
  export_faStarshipFreighter as faStarshipFreighter
};
