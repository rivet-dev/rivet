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

// src/node_modules/@fortawesome/free-brands-svg-icons/faHive.js
var require_faHive = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faHive.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "hive";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e07f";
    var svgPathData = "M260.4 254.9L131.5 33.1c-.2-.3-.5-.6-.8-.8s-.7-.3-1.1-.3-.8 .1-1.1 .3-.6 .5-.8 .8L.3 254.9c-.2 .3-.3 .7-.3 1.1s.1 .8 .3 1.1L129.1 478.9c.2 .3 .5 .6 .8 .8s.7 .3 1.1 .3 .8-.1 1.1-.3 .6-.5 .8-.8L260.4 257.1c.2-.3 .3-.7 .3-1.1s-.1-.8-.3-1.1zm39.1-25.7c.2 .3 .5 .6 .8 .8s.7 .3 1.1 .3l66.5 0c.4 0 .8-.1 1.1-.3s.6-.5 .8-.8 .3-.7 .3-1.1-.1-.8-.3-1.1L259.1 33.1c-.2-.3-.5-.6-.8-.8s-.7-.3-1.1-.3l-66.5 0c-.4 0-.8 .1-1.1 .3s-.6 .5-.8 .8-.3 .7-.3 1.1 .1 .8 .3 1.1L299.4 229.2zm212.3 25.7L384.9 33.1c-.2-.3-.5-.6-.8-.8s-.7-.3-1.1-.3l-66.6 0c-.4 0-.8 .1-1.1 .3s-.6 .5-.8 .8-.3 .7-.3 1.1 .1 .8 .3 1.1L440.7 256 314.5 476.7c-.2 .3-.3 .7-.3 1.1s.1 .8 .3 1.1 .5 .6 .8 .8 .7 .3 1.1 .3l66.6 0c.4 0 .8-.1 1.1-.3s.6-.5 .8-.8L511.7 257.1c.2-.3 .3-.7 .3-1.1s-.1-.8-.3-1.1zM366 284.9l-66.5 0c-.4 0-.8 .1-1.1 .3s-.6 .5-.8 .8L188.8 476.7c-.2 .3-.3 .7-.3 1.1s.1 .8 .3 1.1 .5 .6 .8 .8 .7 .3 1.1 .3l66.5 0c.4 0 .8-.1 1.1-.3s.6-.5 .8-.8L367.9 288.3c.2-.3 .3-.7 .3-1.1s-.1-.8-.3-1.1-.5-.6-.8-.8-.7-.3-1.1-.3z";
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
    exports.faHive = exports.definition;
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

// src/_entries/faHive.js
var import_faHive = __toESM(require_faHive());
var export_faHive = import_faHive.definition;
export {
  export_faHive as faHive
};
