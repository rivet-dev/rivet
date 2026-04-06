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

// src/node_modules/@fortawesome/free-brands-svg-icons/faChromecast.js
var require_faChromecast = __commonJS({
  "src/node_modules/@fortawesome/free-brands-svg-icons/faChromecast.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fab";
    var iconName = "chromecast";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f838";
    var svgPathData = "M448 64L64.2 64c-23.6 0-42.7 19.1-42.7 42.7l0 63.9 42.7 0 0-63.9 383.8 0 0 298.6-149.2 0 0 42.7 149.4 0c23.6 0 42.7-19.1 42.7-42.7l0-298.6C490.9 83.1 471.6 64 448 64zM21.5 383.6l0 63.9 63.9 0c0-35.3-28.6-63.9-63.9-63.9zm0-85l0 42.4c58.9 0 106.6 48.1 106.6 107l42.7 0c.1-82.4-66.9-149.3-149.3-149.4zM213.6 448l42.7 0C255.8 318.5 151 213.7 21.5 213.4l0 42.4c106-.2 192 86.2 192.1 192.2z";
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
    exports.faChromecast = exports.definition;
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

// src/_entries/faChromecast.js
var import_faChromecast = __toESM(require_faChromecast());
var export_faChromecast = import_faChromecast.definition;
export {
  export_faChromecast as faChromecast
};
