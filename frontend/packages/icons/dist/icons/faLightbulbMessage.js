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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faLightbulbMessage.js
var require_faLightbulbMessage = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faLightbulbMessage.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "lightbulb-message";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e687";
    var svgPathData = "M256 288l0 96-144 0c-9.6-31.9-29.5-59.1-49.2-86.2c0 0 0 0 0 0c-5.2-7.1-10.4-14.2-15.4-21.4C27.6 247.9 16 213.3 16 176C16 78.8 94.8 0 192 0s176 78.8 176 176c0 16.6-2.3 32.7-6.6 48L320 224c-35.3 0-64 28.7-64 64zM112 416l144 0c0 14.4 4.8 27.7 12.8 38.4C259.1 487.7 228.4 512 192 512c-44.2 0-80-35.8-80-80l0-16zm0-240c0-44.2 35.8-80 80-80c8.8 0 16-7.2 16-16s-7.2-16-16-16C130.1 64 80 114.1 80 176c0 8.8 7.2 16 16 16s16-7.2 16-16zm432 80c17.7 0 32 14.3 32 32l0 128c0 17.7-14.3 32-32 32l-64 0 0 48c0 6.5-3.9 12.3-9.9 14.8s-12.9 1.1-17.4-3.5L393.4 448 320 448c-17.7 0-32-14.3-32-32l0-128c0-17.7 14.3-32 32-32l224 0z";
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
    exports.faLightbulbMessage = exports.definition;
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

// src/_entries/faLightbulbMessage.js
var import_faLightbulbMessage = __toESM(require_faLightbulbMessage());
var export_faLightbulbMessage = import_faLightbulbMessage.definition;
export {
  export_faLightbulbMessage as faLightbulbMessage
};
