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

// src/node_modules/@fortawesome/free-solid-svg-icons/faBackward.js
var require_faBackward = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faBackward.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "backward";
    var width = 576;
    var height = 512;
    var aliases = [9194];
    var unicode = "f04a";
    var svgPathData = "M204.3 43.1C215.9 32 233 28.9 247.7 35.2S272 56 272 72l0 136.3 172.3-165.1C455.9 32 473 28.9 487.7 35.2S512 56 512 72l0 368c0 16-9.6 30.5-24.3 36.8s-31.8 3.2-43.4-7.9L272 303.7 272 440c0 16-9.6 30.5-24.3 36.8s-31.8 3.2-43.4-7.9l-192-184C4.5 277.3 0 266.9 0 256s4.5-21.3 12.3-28.9l192-184z";
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
    exports.faBackward = exports.definition;
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

// src/_entries/faBackward.js
var import_faBackward = __toESM(require_faBackward());
var export_faBackward = import_faBackward.definition;
export {
  export_faBackward as faBackward
};
