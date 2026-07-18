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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHandcuffs.js
var require_faHandcuffs = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHandcuffs.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "handcuffs";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e4f8";
    var svgPathData = "M320-32c0-17.7-14.3-32-32-32s-32 14.3-32 32 14.3 32 32 32 32-14.3 32-32zM192 64a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM152 96c-13.3 0-24 10.7-24 24l0 16c0 1 .1 1.9 .2 2.9-74.7 26.3-128.2 97.5-128.2 181.1 0 106 86 192 192 192s192-86 192-192c0-83.7-53.5-154.8-128.2-181.1 .1-.9 .2-1.9 .2-2.9l0-16c0-13.3-10.7-24-24-24l-80 0zM64 320a128 128 0 1 1 256 0 128 128 0 1 1 -256 0zm448 0c0 66.9-51.3 121.8-116.6 127.5-14.3 22.8-32.4 43.1-53.4 59.9 13.5 3 27.6 4.6 42 4.6 106 0 192-86 192-192 0-83.7-53.5-154.8-128.2-181.1 .1-.9 .2-1.9 .2-2.9l0-16c0-13.3-10.7-24-24-24l-80 0c-12.3 0-22.4 9.2-23.8 21.1 30.3 19.2 56.1 45 75.2 75.4 65.4 5.8 116.6 60.6 116.6 127.5zM384 64a32 32 0 1 0 0-64 32 32 0 1 0 0 64z";
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
    exports.faHandcuffs = exports.definition;
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

// src/_entries/faHandcuffs.js
var import_faHandcuffs = __toESM(require_faHandcuffs());
var export_faHandcuffs = import_faHandcuffs.definition;
export {
  export_faHandcuffs as faHandcuffs
};
