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

// src/node_modules/@fortawesome/free-brands-svg-icons/faTrello.js
var require_faTrello = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faTrello.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "trello";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "f181";
    var svgPathData = "M392.3 32L56.1 32c-31 0-56.1 25.1-56.1 56-.1 0 0-4 0 336 0 30.9 25.1 56 56 56l336.2 0c30.8-.2 55.7-25.2 55.7-56l0-336c.1-30.8-24.8-55.8-55.6-56zM197 371.3c-.2 14.7-12.1 26.6-26.9 26.6l-82.7 0c-14.8 .1-26.9-11.8-27-26.6l0-254.2c0-14.8 12-26.9 26.9-26.9l82.9 0c14.8 0 26.9 12 26.9 26.9l0 254.2-.1 0zm193.1-112c0 14.8-12 26.9-26.9 26.9l-81 0c-14.8 0-26.9-12-26.9-26.9l0-142.1c0-14.8 12-26.9 26.8-26.9l81.1 0c14.8 0 26.9 12 26.9 26.9l0 142.1z";
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
    exports.faTrello = exports.definition;
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

// src/_entries/faTrello.js
var import_faTrello = __toESM(require_faTrello());
var export_faTrello = import_faTrello.definition;
export {
  export_faTrello as faTrello
};
