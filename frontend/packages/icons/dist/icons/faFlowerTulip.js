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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFlowerTulip.js
var require_faFlowerTulip = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFlowerTulip.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "flower-tulip";
    var width = 512;
    var height = 512;
    var aliases = [127799];
    var unicode = "f801";
    var svgPathData = "M96 44l0 84c0 70.7 57.3 128 128 128l0 138.8C192.1 349.5 139.5 320 80 320l-50.5 0C13.2 320 0 333.2 0 349.5C0 439.3 72.7 512 162.5 512l61.5 0 32 0 32 0 61.5 0C439.3 512 512 439.3 512 349.5c0-16.3-13.2-29.5-29.5-29.5L432 320c-59.5 0-112.1 29.5-144 74.8L288 256c70.7 0 128-57.3 128-128l0-84c0-6.6-5.4-12-12-12c-2.6 0-5.1 .8-7.2 2.4L336 80 268.8 5.3c-3-3.4-7.4-5.3-11.9-5.3l-1.7 0c-4.5 0-8.9 1.9-11.9 5.3L176 80 115.2 34.4c-2.1-1.6-4.6-2.4-7.2-2.4c-6.6 0-12 5.4-12 12z";
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
    exports.faFlowerTulip = exports.definition;
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

// src/_entries/faFlowerTulip.js
var import_faFlowerTulip = __toESM(require_faFlowerTulip());
var export_faFlowerTulip = import_faFlowerTulip.definition;
export {
  export_faFlowerTulip as faFlowerTulip
};
