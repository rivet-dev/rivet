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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAvocado.js
var require_faAvocado = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAvocado.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "avocado";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e0aa";
    var svgPathData = "M459.4 255.3c-18.5 15.2-33.5 35.6-38.1 59.1c-10.2 51.9-34 104.5-66.3 136.7c-81.2 81.2-212.9 81.2-294.2 0s-81.2-212.9 0-294.2c32.3-32.3 84.9-56.1 136.7-66.3c23.5-4.6 43.9-19.6 59.1-38.1C283.1 20.5 323.2 0 368 0c79.5 0 144 64.5 144 144c0 44.8-20.5 84.9-52.6 111.3zM283.2 347.2c41.7-41.7 49.1-102.1 16.4-134.8s-93-25.3-134.8 16.4s-49.1 102.1-16.4 134.8s93 25.3 134.8-16.4z";
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
    exports.faAvocado = exports.definition;
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

// src/_entries/faAvocado.js
var import_faAvocado = __toESM(require_faAvocado());
var export_faAvocado = import_faAvocado.definition;
export {
  export_faAvocado as faAvocado
};
